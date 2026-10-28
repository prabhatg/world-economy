//! Mechanisms: pluggable units of model behavior.
//!
//! A mechanism declares its [`MechanismSpec`] (name, period, components read and
//! written, ordering) and implements the decide phase for one partition. The engine
//! owns everything else: partitioning, migrant exchange, canonical ordering, merging
//! and recounting. Mechanisms see only the read-only [`Sense`] snapshot and their own
//! partition, so they parallelize across partitions without locks and stay
//! partition-independent by construction.
//!
//! The engine calls each due mechanism tile by tile: a tile is a run of agents ending
//! on a cell boundary, small enough to stay in cache, and every due mechanism processes
//! it before the next tile starts. Mechanisms must therefore be local to the cells of
//! the tile they are given (reading anything else only through [`Sense`]).

use crate::agents::Agents;
use crate::params::Params;
use crate::schedule::{Component, MechanismSpec};
use crate::world::StepStats;
use oikos_core::prob::stochastic_round;
use oikos_core::{EntityId, Philox4x32, Probability, Stream, Tick};
use oikos_grid::Grid;
use std::ops::Range;

pub trait Mechanism: Send + Sync {
    fn spec(&self) -> MechanismSpec;

    /// Decide phase for the agents at indices `tile` (whole cells) of one partition.
    /// May update those agents in place and spawn agents through
    /// [`PartitionView::spawn`]; spawned agents are not part of any tile this step.
    fn decide(&self, sense: &Sense<'_>, part: &mut PartitionView<'_>, tile: Range<usize>);
}

/// Read-only world state for the decide phase (the halo in a distributed run).
pub struct Sense<'a> {
    pub grid: &'a Grid,
    /// Forcing-pack fields (declare [`Component::Forcing`] to read them).
    pub inputs: &'a crate::inputs::Inputs,
    pub capacity: &'a [u64],
    /// Population per cell after the previous step's commit.
    pub population: &'a [u64],
    pub tick: Tick,
    pub seed: u64,
}

/// One partition's mutable state during the decide phase.
pub struct PartitionView<'a> {
    pub cells: Range<u32>,
    pub agents: &'a mut Agents,
    pub stats: &'a mut StepStats,
    pub(crate) counters: &'a mut [u64],
    pub(crate) outbox: &'a mut [Agents],
    pub(crate) ranges: &'a [Range<u32>],
}

impl PartitionView<'_> {
    /// Creates an agent born in `birth_cell` (which this partition owns) and placed in
    /// `dest`. Ids come from the birth cell's counter, so they never depend on
    /// partitioning. Agents placed in another cell count as migrants.
    pub fn spawn(&mut self, birth_cell: u32, dest: u32, weight: u64) -> EntityId {
        let counter = &mut self.counters[(birth_cell - self.cells.start) as usize];
        let id = EntityId::new(birth_cell, *counter);
        *counter += 1;
        if dest != birth_cell {
            self.stats.migrants += weight;
        }
        if self.cells.contains(&dest) {
            self.agents.push(id, dest, weight);
        } else {
            let owner = self.ranges.partition_point(|r| r.end <= dest);
            self.outbox[owner].push(id, dest, weight);
        }
        id
    }
}

/// The M0 toy mechanisms, configured from `params`, in registration order.
pub fn defaults(params: &Params) -> Vec<Box<dyn Mechanism>> {
    vec![Box::new(Growth::from_params(params)), Box::new(Fission::from_params(params))]
}

/// Logistic growth toward each cell's carrying capacity: dw = r w (1 - N/K),
/// stochastically rounded to whole people.
#[derive(Clone, Debug)]
pub struct Growth {
    pub period_ticks: u64,
    pub rate: f64,
}

impl Growth {
    pub fn from_params(p: &Params) -> Growth {
        Growth { period_ticks: p.period_ticks, rate: p.growth_rate }
    }
}

impl Mechanism for Growth {
    fn spec(&self) -> MechanismSpec {
        MechanismSpec {
            name: "growth",
            period_ticks: self.period_ticks,
            reads: vec![Component::Population, Component::Capacity],
            writes: vec![Component::AgentWeight],
            after: vec![],
        }
    }

    fn decide(&self, sense: &Sense<'_>, part: &mut PartitionView<'_>, tile: Range<usize>) {
        let rng = Philox4x32::new(sense.seed, Stream::Growth);
        let t = sense.tick.0;
        let agents = &mut *part.agents;
        // Count in locals: writing shared stats per agent would cause cache-line traffic.
        let (mut births, mut deaths) = (0, 0);
        for i in tile {
            let cell = agents.cell[i] as usize;
            let w = agents.weight[i];
            let pop = sense.population[cell] as f64;
            let k = sense.capacity[cell] as f64;
            let dw = oikos_math::finite(self.rate * w as f64 * (1.0 - pop / k));
            let draw = rng.u32(agents.id[i].0, t, 0, 0);
            if dw >= 0.0 {
                let b = stochastic_round(dw, draw);
                agents.weight[i] = w + b;
                births += b;
            } else {
                let d = stochastic_round(-dw, draw).min(w);
                agents.weight[i] = w - d;
                deaths += d;
            }
        }
        part.stats.births += births;
        part.stats.deaths += deaths;
    }
}

/// Band fission: large bands split in half with some probability; the daughter moves
/// to the least crowded neighbor (by N/K), or explores a random neighbor.
#[derive(Clone, Debug)]
pub struct Fission {
    pub period_ticks: u64,
    pub threshold: u64,
    pub probability: f64,
    pub exploration: f64,
    pub after: Vec<&'static str>,
}

impl Fission {
    pub fn from_params(p: &Params) -> Fission {
        Fission {
            period_ticks: p.period_ticks,
            threshold: p.fission_threshold,
            probability: p.fission_probability,
            exploration: p.exploration_probability,
            after: vec!["growth"],
        }
    }

    fn destination(
        &self,
        sense: &Sense<'_>,
        rng: &Philox4x32,
        explore: Probability,
        id: EntityId,
        cell: u32,
    ) -> u32 {
        let ns = sense.grid.neighbors(cell);
        if ns.is_empty() {
            return cell;
        }
        let draws = rng.block(id.0, sense.tick.0, 0);
        if explore.hit(draws[0]) {
            return ns[(((draws[1] as u64) * (ns.len() as u64)) >> 32) as usize];
        }
        // Compare N_a / K_a < N_b / K_b exactly as N_a K_b < N_b K_a.
        let crowd = |c: u32| (sense.population[c as usize] as u128, sense.capacity[c as usize] as u128);
        let mut best = cell;
        let (mut bn, mut bk) = crowd(cell);
        for &c in ns {
            let (n, k) = crowd(c);
            if n * bk < bn * k {
                best = c;
                (bn, bk) = (n, k);
            }
        }
        best
    }
}

impl Mechanism for Fission {
    fn spec(&self) -> MechanismSpec {
        MechanismSpec {
            name: "fission",
            period_ticks: self.period_ticks,
            reads: vec![Component::Population, Component::Capacity],
            writes: vec![Component::AgentWeight, Component::AgentSpawn],
            after: self.after.clone(),
        }
    }

    fn decide(&self, sense: &Sense<'_>, part: &mut PartitionView<'_>, tile: Range<usize>) {
        let split = Probability::from_f64(self.probability);
        let explore = Probability::from_f64(self.exploration);
        let fission_rng = Philox4x32::new(sense.seed, Stream::Fission);
        let migration_rng = Philox4x32::new(sense.seed, Stream::Migration);
        let t = sense.tick.0;
        for i in tile {
            // Weight first: most agents are below the threshold, so skip them after one load.
            let mut w = part.agents.weight[i];
            if w < self.threshold {
                continue;
            }
            let id = part.agents.id[i];
            if !split.hit(fission_rng.u32(id.0, t, 0, 0)) {
                continue;
            }
            let cell = part.agents.cell[i];
            let take = w / 2;
            let half = Agents::split_weight(&mut w, take);
            part.agents.weight[i] = w;
            part.stats.fissions += 1;
            let dest = self.destination(sense, &migration_rng, explore, id, cell);
            part.spawn(cell, dest, half);
        }
    }
}
