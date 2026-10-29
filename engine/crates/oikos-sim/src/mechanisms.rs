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

use crate::agents::{Agents, Traits};
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
    /// Carrying capacity per cell: what foraging and the bands' cultivation feed.
    pub capacity: &'a [u64],
    /// Forager carrying capacity per cell (from the climate; equal to `capacity` in
    /// worlds without one).
    pub forage_capacity: &'a [u64],
    /// Whether the forcing supports cultivation (a climate pack); without it the fields
    /// below are empty.
    pub farming: bool,
    /// Wild-cereal suitability per cell, in [0, 1] (declare [`Component::Forcing`]).
    pub suitability: &'a [f64],
    /// Relative decline of forager capacity over the stress window, in [0, 1].
    pub climate_stress: &'a [f64],
    /// Mean change of suitability per century over the stress window.
    pub climate_variability: &'a [f64],
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
    /// Creates an agent with `traits` born in `birth_cell` (which this partition owns) and
    /// placed in `dest`. Ids come from the birth cell's counter, so they never depend on
    /// partitioning. Agents placed in another cell count as migrants.
    pub fn spawn(&mut self, birth_cell: u32, dest: u32, weight: u64, traits: Traits) -> EntityId {
        let counter = &mut self.counters[(birth_cell - self.cells.start) as usize];
        let id = EntityId::new(birth_cell, *counter);
        *counter += 1;
        if dest != birth_cell {
            self.stats.migrants += weight;
        }
        if self.cells.contains(&dest) {
            self.agents.push_with(id, dest, weight, traits);
        } else {
            let owner = self.ranges.partition_point(|r| r.end <= dest);
            self.outbox[owner].push_with(id, dest, weight, traits);
        }
        id
    }
}

/// The model's mechanisms, configured from `params`, in registration order.
pub fn defaults(params: &Params) -> Vec<Box<dyn Mechanism>> {
    vec![
        Box::new(Growth::from_params(params)),
        Box::new(Fission::from_params(params)),
        Box::new(Dispersal::from_params(params)),
        Box::new(Inundation::from_params(params)),
        Box::new(Cultivation::from_params(params)),
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
///
/// The intrinsic rate r is the Neolithic demographic transition (Bocquet-Appel 2002,
/// 2011): it moves linearly with a band's cultivation effort from the mobile foragers'
/// rate to the sedentary cultivators'. Sedentism and cereal weaning foods shorten birth
/// intervals (Sellen and Mace 1997; Page et al. 2016); the net effect is what the rates
/// encode. Foragers' bands (and toy worlds, which never cultivate) grow at `rate`.
#[derive(Clone, Debug)]
pub struct Growth {
    pub period_ticks: u64,
    pub rate: f64,
    pub sedentary_rate: f64,
}

impl Growth {
    pub fn from_params(p: &Params) -> Growth {
        Growth { period_ticks: p.period_ticks, rate: p.growth_rate, sedentary_rate: p.sedentary_growth_rate }
    }

    /// Intrinsic growth rate of a band spending `effort` of its effort cultivating.
    pub fn rate(&self, effort: f64) -> f64 {
        self.rate + (self.sedentary_rate - self.rate) * effort
    }
}

impl Mechanism for Growth {
    fn spec(&self) -> MechanismSpec {
        MechanismSpec {
            name: "growth",
            period_ticks: self.period_ticks,
            reads: vec![Component::Population, Component::Capacity, Component::AgentTraits],
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
            let r = self.rate(Traits::share(agents.cultivation[i]));
            let dw = oikos_math::finite(r * w as f64 * (1.0 - pop / k));
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
            reads: vec![
                Component::Population,
                Component::Capacity,
                Component::Forcing,
                Component::AgentTraits,
            ],
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
            let traits = part.agents.traits(i);
            part.spawn(cell, dest, half, traits);
        }
    }
}

/// Forager dispersal: a cell at least `crowding` full (N/K, previous step) buds off one
/// group with probability `probability` per period: `fraction` of its first band whose
/// share fits moves to the least crowded habitable neighbor, if that neighbor is less
/// crowded and can feed the group (its population, last step, plus the group within its
/// capacity). A res-5 cell is about one band's territory. Repeated, this is
/// the wave of advance by which foragers fill a landscape (Fisher-KPP; Ammerman and
/// Cavalli-Sforza 1984), and it works where a cell holds a single band, so fission by
/// group size never fires (res-5 cells feed ~20-50 foragers).
#[derive(Clone, Debug)]
pub struct Dispersal {
    pub period_ticks: u64,
    pub crowding: f64,
    pub probability: f64,
    pub fraction: f64,
    pub after: Vec<&'static str>,
}

impl Dispersal {
    /// A habitable cell at least `crowding` full (last step).
    fn crowded(&self, sense: &Sense<'_>, cell: u32) -> bool {
        let (n, k) = (sense.population[cell as usize], sense.capacity[cell as usize]);
        sense.habitable[cell as usize] && n as f64 >= self.crowding * k as f64
    }

    /// The least crowded habitable neighbor of `cell`, if it is less crowded than `cell`,
    /// and its room (capacity less population, last step).
    fn target(&self, sense: &Sense<'_>, cell: u32) -> Option<(u32, u64)> {
        let (n, k) = (sense.population[cell as usize], sense.capacity[cell as usize]);
        let dest = least_crowded(sense, habitable_neighbors(sense, cell))?;
        let (nd, kd) = (sense.population[dest as usize], sense.capacity[dest as usize]);
        (nd as u128 * (k as u128) < n as u128 * kd as u128 && nd < kd).then(|| (dest, kd - nd))
    }

    pub fn from_params(p: &Params) -> Dispersal {
        Dispersal {
            period_ticks: p.period_ticks,
            crowding: p.dispersal_crowding,
            probability: p.dispersal_probability,
            fraction: p.dispersal_fraction,
            after: vec!["fission"],
        }
    }
}

impl Mechanism for Dispersal {
    fn spec(&self) -> MechanismSpec {
        MechanismSpec {
            name: "dispersal",
            period_ticks: self.period_ticks,
            reads: vec![
                Component::Population,
                Component::Capacity,
                Component::Forcing,
                Component::AgentTraits,
            ],
            writes: vec![Component::AgentWeight, Component::AgentSpawn],
            after: self.after.clone(),
        }
    }

    fn decide(&self, sense: &Sense<'_>, part: &mut PartitionView<'_>, tile: Range<usize>) {
        let go = Probability::from_f64(self.probability);
        let rng = Philox4x32::new(sense.seed, Stream::Dispersal);
        let t = sense.tick.0;
        // Tiles hold whole cells; agents spawned this step lie past the tile.
        let mut i = tile.start;
        while i < tile.end {
            let cell = part.agents.cell[i];
            let end = i + part.agents.cell[i..tile.end].iter().take_while(|&&c| c == cell).count();
            if self.crowded(sense, cell)
                && go.hit(rng.u32(cell as u64, t, 0, 0))
                && let Some((dest, room)) = self.target(sense, cell)
            {
                // The first band (canonical order) whose group fits buds off.
                for a in i..end {
                    let w = part.agents.weight[a];
                    let take = (w as f64 * self.fraction) as u64;
                    if take > 0 && take <= room {
                        part.agents.weight[a] = w - take;
                        let traits = part.agents.traits(a);
                        part.spawn(cell, dest, take, traits);
                        break;
                    }
                }
            }
            i = end;
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
        Inundation { period_ticks: p.period_ticks, after: vec!["growth", "fission", "dispersal"] }
    }
}

impl Mechanism for Inundation {
    fn spec(&self) -> MechanismSpec {
        MechanismSpec {
            name: "inundation",
            period_ticks: self.period_ticks,
            reads: vec![
                Component::Population,
                Component::Capacity,
                Component::Forcing,
                Component::AgentTraits,
            ],
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
                    let traits = part.agents.traits(i);
                    part.spawn(cell, dest, w, traits);
                }
                None => lost += w,
            }
        }
        part.stats.deaths += lost;
    }
}

/// Cultivation and domestication as co-evolution (docs/04 §4.2, ADR-0049), per band and
/// period:
///
/// - **Effort.** A band moves its share of effort spent cultivating wild cereals toward a
///   target, at `adoption_rate` (social learning). The target is zero where wild cereals
///   do not grow; elsewhere it adds three drivers the literature disagrees on, each with
///   its own weight, so calibration can weigh them against the record: *population
///   pressure* (people beyond `crowding` times forager capacity, full at one capacity
///   more: foragers at equilibrium feel none, but a climate that cuts capacity, or
///   cultivation that lets people outgrow it, does; Binford 1968, Cohen 1977),
///   *climate stress* (the decline of forager capacity over the stress window, e.g. the
///   Younger Dryas; Moore and Hillman 1992, Bar-Yosef 1998) and *abundance* (rich wild
///   stands invite cultivation without stress; Smith 2007, Zeder 2016). A fourth
///   hypothesis scales the target instead of adding to it: where suitability swings
///   from century to century, sowing does not pay, whatever the motive (the stable
///   Holocene climate made agriculture possible; Richerson, Boyd and Bettinger 2001).
/// - **Domestication.** Harvesting and resowing select for domestic traits
///   (non-shattering ears) at `selection` times effort, logistically in their share of
///   the sown crop (from at least the wild mutant frequency); gene flow from wild stands
///   and abandoned fields pulls the share back at `introgression` times the effort not
///   spent cultivating. Weak selection gives the protracted domestication seen in the
///   archaeobotanical record (Purugganan and Fuller 2011; Allaby et al. 2008); strong
///   selection the rapid one measured in experimental cultivation (Hillman and Davies 1990).
///
/// Cultivation feeds back through carrying capacity ([`crate::environment::farmed_capacity`]):
/// domestic crops feed more people per area, so populations grow, pressure rises and
/// effort with it (Rindos 1984). Toy worlds (no climate pack) never cultivate.
#[derive(Clone, Debug)]
pub struct Cultivation {
    pub period_ticks: u64,
    pub crowding: f64,
    pub pressure_weight: f64,
    pub stress_weight: f64,
    pub abundance_weight: f64,
    pub variability_weight: f64,
    pub adoption_rate: f64,
    pub selection: f64,
    pub wild_frequency: f64,
    pub introgression: f64,
    pub after: Vec<&'static str>,
}

impl Cultivation {
    pub fn from_params(p: &Params) -> Cultivation {
        Cultivation {
            period_ticks: p.period_ticks,
            crowding: p.cultivation_crowding,
            pressure_weight: p.cultivation_pressure_weight,
            stress_weight: p.cultivation_stress_weight,
            abundance_weight: p.cultivation_abundance_weight,
            variability_weight: p.cultivation_variability_weight,
            adoption_rate: p.adoption_rate,
            selection: p.selection_coefficient,
            wild_frequency: p.domestic_wild_frequency,
            introgression: p.introgression_rate,
            after: vec!["inundation"],
        }
    }

    /// The share of effort bands in a cell aim to spend cultivating: `people` against
    /// `forage` capacity, wild-cereal `suitability`, climate `stress`, and climate
    /// `variability`, which makes harvests unreliable whatever the motive.
    pub fn target_effort(
        &self,
        people: u64,
        forage: u64,
        suitability: f64,
        stress: f64,
        variability: f64,
    ) -> f64 {
        if suitability <= 0.0 {
            return 0.0;
        }
        let crowding = people as f64 / forage.max(1) as f64;
        let pressure = (crowding - self.crowding).clamp(0.0, 1.0);
        let drive = self.pressure_weight * pressure
            + self.stress_weight * stress
            + self.abundance_weight * suitability;
        let reliability = (1.0 - self.variability_weight * variability).clamp(0.0, 1.0);
        oikos_math::finite(drive * reliability).clamp(0.0, 1.0)
    }

    /// Effort after one period of moving from `effort` toward `target`.
    pub fn adopt(&self, effort: f64, target: f64) -> f64 {
        effort + self.adoption_rate * (target - effort)
    }

    /// Domestic share of the crop after one period at `effort`, from `d`.
    pub fn domesticate(&self, d: f64, effort: f64) -> f64 {
        let gain = self.selection * effort * d.max(self.wild_frequency) * (1.0 - d);
        let loss = self.introgression * (1.0 - effort) * d;
        oikos_math::finite(d + gain - loss).clamp(0.0, 1.0)
    }
}

impl Mechanism for Cultivation {
    fn spec(&self) -> MechanismSpec {
        MechanismSpec {
            name: "cultivation",
            period_ticks: self.period_ticks,
            reads: vec![
                Component::Population,
                Component::Capacity,
                Component::Forcing,
                Component::AgentTraits,
            ],
            writes: vec![Component::AgentTraits],
            after: self.after.clone(),
        }
    }

    fn decide(&self, sense: &Sense<'_>, part: &mut PartitionView<'_>, tile: Range<usize>) {
        if !sense.farming {
            return;
        }
        let agents = &mut *part.agents;
        for i in tile {
            let cell = agents.cell[i] as usize;
            let traits = agents.traits(i);
            let target = self.target_effort(
                sense.population[cell],
                sense.forage_capacity[cell],
                sense.suitability[cell],
                sense.climate_stress[cell],
                sense.climate_variability[cell],
            );
            if target == 0.0 && traits == Traits::FORAGER {
                continue;
            }
            let (e, d) = (Traits::share(traits.cultivation), Traits::share(traits.domestication));
            agents.set_traits(
                i,
                Traits {
                    cultivation: Traits::toward(traits.cultivation, self.adopt(e, target)),
                    domestication: Traits::toward(traits.domestication, self.domesticate(d, e)),
                },
            );
        }
    }
}
