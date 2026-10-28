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
    /// Whether each cell is habitable now (see [`crate::inputs::Inputs::habitable_into`]);
    /// derived from forcing, so readers declare [`Component::Forcing`].
    pub habitable: &'a [bool],
    /// Whether forcing packs restrict `habitable` (false: every cell is habitable).
    pub restricted: bool,
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
    vec![
        Box::new(Growth::from_params(params)),
        Box::new(Fission::from_params(params)),
        Box::new(Inundation::from_params(params)),
    ]
}

/// The least crowded of `cells` by N/K (the first on ties), compared exactly as
/// N_a K_b < N_b K_a.
fn least_crowded(sense: &Sense<'_>, cells: impl IntoIterator<Item = u32>) -> Option<u32> {
    let crowd = |c: u32| (sense.population[c as usize] as u128, sense.capacity[c as usize] as u128);
    let mut best: Option<(u32, u128, u128)> = None;
    for c in cells {
        let (n, k) = crowd(c);
        if best.is_none_or(|(_, bn, bk)| n * bk < bn * k) {
            best = Some((c, n, k));
        }
    }
    best.map(|(c, _, _)| c)
}

fn habitable_neighbors<'a>(sense: &'a Sense<'_>, cell: u32) -> impl Iterator<Item = u32> + Clone + 'a {
    sense.grid.neighbors(cell).iter().copied().filter(|&c| sense.habitable[c as usize])
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
        let ns = habitable_neighbors(sense, cell);
        let open = ns.clone().count();
        if open == 0 {
            return cell;
        }
        let draws = rng.block(id.0, sense.tick.0, 0);
        if explore.hit(draws[0]) {
            return ns.clone().nth((((draws[1] as u64) * (open as u64)) >> 32) as usize).unwrap();
        }
        least_crowded(sense, std::iter::once(cell).chain(ns)).unwrap()
    }
}

impl Mechanism for Fission {
    fn spec(&self) -> MechanismSpec {
        MechanismSpec {
            name: "fission",
            period_ticks: self.period_ticks,
            reads: vec![Component::Population, Component::Capacity, Component::Forcing],
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
            let cell = part.agents.cell[i];
            // Bands the sea has flooded do not split; inundation moves them this step.
            if !sense.habitable[cell as usize] {
                continue;
            }
            let id = part.agents.id[i];
            if !split.hit(fission_rng.u32(id.0, t, 0, 0)) {
                continue;
            }
            let take = w / 2;
            let half = Agents::split_weight(&mut w, take);
            part.agents.weight[i] = w;
            part.stats.fissions += 1;
            let dest = self.destination(sense, &migration_rng, explore, id, cell);
            part.spawn(cell, dest, half);
        }
    }
}

/// Bands in a cell the sea has flooded move to the least crowded habitable neighbor
/// (by N/K) as new agents born there, or are lost if no neighbor is habitable. A
/// res-5 cell floods only when none of it is land, so a neighbor is usually open.
/// Without coastline or hydrology packs every cell is habitable and this does nothing.
#[derive(Clone, Debug)]
pub struct Inundation {
    pub period_ticks: u64,
    pub after: Vec<&'static str>,
}

impl Inundation {
    pub fn from_params(p: &Params) -> Inundation {
        Inundation { period_ticks: p.period_ticks, after: vec!["growth", "fission"] }
    }
}

impl Mechanism for Inundation {
    fn spec(&self) -> MechanismSpec {
        MechanismSpec {
            name: "inundation",
            period_ticks: self.period_ticks,
            reads: vec![Component::Population, Component::Capacity, Component::Forcing],
            writes: vec![Component::AgentWeight, Component::AgentSpawn],
            after: self.after.clone(),
        }
    }

    fn decide(&self, sense: &Sense<'_>, part: &mut PartitionView<'_>, tile: Range<usize>) {
        if !sense.restricted {
            return;
        }
        let mut lost = 0;
        for i in tile {
            let (cell, w) = (part.agents.cell[i], part.agents.weight[i]);
            if sense.habitable[cell as usize] || w == 0 {
                continue;
            }
            part.agents.weight[i] = 0;
            match least_crowded(sense, habitable_neighbors(sense, cell)) {
                Some(dest) => {
                    part.spawn(cell, dest, w);
                }
                None => lost += w,
            }
        }
        part.stats.deaths += lost;
    }
}
