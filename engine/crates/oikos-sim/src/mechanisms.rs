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

use crate::agents::{Agents, SHARE_ONE, Traits};
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
    /// People-weighted sum of the agents' cultivation effort per cell (in
    /// [`crate::agents::SHARE_ONE`] units), after the previous step's commit.
    pub effort_sum: &'a [u64],
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
        Box::new(Disease::from_params(params)),
        Box::new(Fission::from_params(params)),
        Box::new(Dispersal::from_params(params)),
        Box::new(Inundation::from_params(params)),
        Box::new(Aggregation::from_params(params)),
        Box::new(Cultivation::from_params(params)),
        Box::new(Inequality::from_params(params)),
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

/// Logistic growth toward each cell's carrying capacity, dw = r w (1 - N/K), as gross
/// births and deaths, each stochastically rounded to whole people.
///
/// The intrinsic rate r is the Neolithic demographic transition (Bocquet-Appel 2002,
/// 2011): it moves linearly with a band's cultivation effort from the mobile foragers'
/// rate to the sedentary cultivators'. Sedentism and cereal weaning foods shorten birth
/// intervals (Sellen and Mace 1997; Page et al. 2016); the net effect is what the rates
/// encode. Foragers' bands (and toy worlds, which never cultivate) grow at `rate`.
///
/// Births are the band's fertility (crude birth rate), which moves with effort in the
/// same way; deaths are the rest of the change, so the net is still logistic (ADR-0068).
/// Fertility is what the skeletal record sees (the juvenile index), and at capacity it
/// changes births and deaths, not numbers. Net growth beyond fertility is capped at it.
/// Fertility 0 (forager and sedentary) is the growth before ADR-0068: the net change
/// alone, rounded once, which checkpoints from before it restore with (ADR-0069).
#[derive(Clone, Debug)]
pub struct Growth {
    pub period_ticks: u64,
    pub rate: f64,
    pub sedentary_rate: f64,
    pub fertility: f64,
    pub sedentary_fertility: f64,
}

impl Growth {
    pub fn from_params(p: &Params) -> Growth {
        Growth {
            period_ticks: p.period_ticks,
            rate: p.growth_rate,
            sedentary_rate: p.sedentary_growth_rate,
            fertility: p.fertility,
            sedentary_fertility: p.sedentary_fertility,
        }
    }

    /// Intrinsic growth rate of a band spending `effort` of its effort cultivating.
    pub fn rate(&self, effort: f64) -> f64 {
        self.rate + (self.sedentary_rate - self.rate) * effort
    }

    /// Crude birth rate of a band spending `effort` of its effort cultivating.
    pub fn fertility(&self, effort: f64) -> f64 {
        self.fertility + (self.sedentary_fertility - self.fertility) * effort
    }

    /// Births and deaths per person and period of a band at `effort` in a cell of `pop`
    /// people and capacity `k`.
    pub fn vital_rates(&self, effort: f64, pop: f64, k: f64) -> (f64, f64) {
        let b = self.fertility(effort);
        (b, (b - self.rate(effort) * (1.0 - pop / k)).max(0.0))
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
        let net_only = self.fertility == 0.0 && self.sedentary_fertility == 0.0;
        let (mut births, mut deaths) = (0, 0);
        for i in tile {
            let cell = agents.cell[i] as usize;
            let w = agents.weight[i];
            let pop = sense.population[cell] as f64;
            let k = sense.capacity[cell] as f64;
            let effort = Traits::share(agents.cultivation[i]);
            let draw = rng.block(agents.id[i].0, t, 0);
            let (born, died) = if net_only {
                let dw = oikos_math::finite(self.rate(effort) * w as f64 * (1.0 - pop / k));
                if dw >= 0.0 {
                    (stochastic_round(dw, draw[0]), 0)
                } else {
                    (0, stochastic_round(-dw, draw[0]).min(w))
                }
            } else {
                let (b, d) = self.vital_rates(effort, pop, k);
                let born = stochastic_round(oikos_math::finite(b * w as f64), draw[0]);
                (born, stochastic_round(oikos_math::finite(d * w as f64), draw[1]).min(w + born))
            };
            agents.weight[i] = w + born - died;
            births += born;
            deaths += died;
        }
        part.stats.births += births;
        part.stats.deaths += deaths;
    }
}

/// Endemic infection within groups (ADR-0075), a density-dependent disease burden.
/// Transmission rises with a group's size, so an infection persists only in groups above a
/// critical size, and in those at prevalence 1 - 1/R0, R0 being the size over the critical
/// size (Anderson and May 1991; critical community size, Bartlett 1957): the people above
/// the critical size are infected, and die at `mortality` per period. It is the burden
/// settlement adds, infection spread by crowding, waste and fouled water; foragers' own
/// persistent infections, endemic even in isolated bands (Black 1975), are in their
/// baseline mortality, so a camp below its critical size carrying none is an assumption.
///
/// The drivers are behavioral and social. Mobile foragers move camp as waste, vermin and
/// fouled water build up, acting on disgust, the behavioral immune system (Kelly 2013;
/// Schaller and Park 2011; Curtis et al. 2011); settled farmers stay with their fields,
/// stores and houses, among their waste and each other (Armelagos et al. 1991). So a
/// group's contact moves linearly with its cultivation effort from `mobile_contact` to 1,
/// and its critical size is `threshold` over its contact: foragers' camps add none, and
/// the large villages that defence, exchange and marriage draw people into (ADR-0071) pay
/// for their size in infection (Eshed et al. 2010). `mortality` 0 switches it off.
#[derive(Clone, Debug)]
pub struct Disease {
    pub period_ticks: u64,
    pub threshold: f64,
    pub mobile_contact: f64,
    pub mortality: f64,
    pub after: Vec<&'static str>,
}

impl Disease {
    pub fn from_params(p: &Params) -> Disease {
        Disease {
            period_ticks: p.period_ticks,
            threshold: p.disease_threshold,
            mobile_contact: p.mobile_contact,
            mortality: p.disease_mortality,
            after: vec!["growth"],
        }
    }

    /// Transmission in a group at cultivation `effort`, relative to sedentary villagers'.
    pub fn contact(&self, effort: f64) -> f64 {
        self.mobile_contact + (1.0 - self.mobile_contact) * effort
    }

    /// The smallest group at `effort` in which the infection persists (infinite for
    /// foragers without contact).
    pub fn critical_size(&self, effort: f64) -> f64 {
        self.threshold / self.contact(effort)
    }

    /// Endemic prevalence in a group of `people` at `effort`.
    pub fn prevalence(&self, people: f64, effort: f64) -> f64 {
        (1.0 - self.critical_size(effort) / people).max(0.0)
    }

    /// Expected deaths per period from infection in a group of `people` at `effort`.
    pub fn deaths(&self, people: f64, effort: f64) -> f64 {
        self.mortality * (people - self.critical_size(effort)).max(0.0)
    }
}

impl Mechanism for Disease {
    fn spec(&self) -> MechanismSpec {
        MechanismSpec {
            name: "disease",
            period_ticks: self.period_ticks,
            reads: vec![Component::AgentWeight, Component::AgentTraits],
            writes: vec![Component::AgentWeight],
            after: self.after.clone(),
        }
    }

    fn decide(&self, sense: &Sense<'_>, part: &mut PartitionView<'_>, tile: Range<usize>) {
        if self.mortality == 0.0 {
            return;
        }
        let rng = Philox4x32::new(sense.seed, Stream::Disease);
        let t = sense.tick.0;
        let agents = &mut *part.agents;
        // No group this small carries the infection, whatever its practice: most are skipped.
        let carriers = self.critical_size(1.0) as u64;
        let mut deaths = 0;
        for i in tile {
            let w = agents.weight[i];
            if w <= carriers {
                continue;
            }
            let expected = self.deaths(w as f64, Traits::share(agents.cultivation[i]));
            if expected == 0.0 {
                continue;
            }
            let died =
                stochastic_round(oikos_math::finite(expected), rng.u32(agents.id[i].0, t, 0, 0)).min(w);
            agents.weight[i] = w - died;
            deaths += died;
        }
        part.stats.deaths += deaths;
    }
}

/// Fission by scalar stress: a group above its size limit splits in half with some
/// probability per period (Johnson 1982: decisions among too many units overload a group,
/// which splits unless it builds hierarchy). The limit rises with the group's cultivation
/// effort, in proportion, from mobile foragers' camps (`threshold`) to sedentary farming
/// villages (`village`; Alberti 2014, Kosse 1990), so farmers live in villages and
/// foragers in camps (ADR-0070). The daughter founds a new settlement in the group's own
/// cell while its land feeds more people (population below capacity, last step), and
/// otherwise goes to the least crowded of its cell and neighbors (by N/K), or explores a
/// random neighbor.
///
/// `village` 0 is the fission before villages: one limit, `threshold`, and the daughter
/// always placed by crowding or exploration, room or not.
#[derive(Clone, Debug)]
pub struct Fission {
    pub period_ticks: u64,
    pub threshold: u64,
    pub village: u64,
    pub probability: f64,
    pub exploration: f64,
    pub after: Vec<&'static str>,
}

impl Fission {
    pub fn from_params(p: &Params) -> Fission {
        Fission {
            period_ticks: p.period_ticks,
            threshold: p.fission_threshold,
            village: p.village_fission_size,
            probability: p.fission_probability,
            exploration: p.exploration_probability,
            after: vec!["disease"],
        }
    }

    /// The smallest group any limit lets split (at least two people).
    fn smallest(&self) -> u64 {
        (if self.village == 0 { self.threshold } else { self.threshold.min(self.village) }).max(2)
    }

    /// Whether a group of `weight` people spending `effort` cultivating is above its limit.
    pub fn splits(&self, weight: u64, effort: f64) -> bool {
        weight >= self.smallest() && (self.village == 0 || weight as f64 >= self.limit(effort))
    }

    /// Size limit of a group spending `effort` of its effort cultivating.
    pub fn limit(&self, effort: f64) -> f64 {
        if self.village == 0 || effort == 0.0 {
            return self.threshold as f64;
        }
        if effort == 1.0 {
            return self.village as f64;
        }
        let (camp, village) = (oikos_math::ln(self.threshold as f64), oikos_math::ln(self.village as f64));
        oikos_math::exp(camp + (village - camp) * effort)
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
        let smallest = self.smallest();
        for i in tile {
            // Weight first: most agents are below every limit, so skip them after one load.
            let mut w = part.agents.weight[i];
            if w < smallest || !self.splits(w, Traits::share(part.agents.cultivation[i])) {
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
            let c = cell as usize;
            let room = self.village != 0 && sense.population[c] < sense.capacity[c];
            let dest = if room { cell } else { self.destination(sense, &migration_rng, explore, id, cell) };
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

/// Aggregation of farming villages (ADR-0071): within a cell, a group joins with
/// probability `rate` times its cultivation effort per period the largest group at least
/// its own size that has room for it, one whose joined size stays below the scalar-stress
/// limit ([`Fission::limit`]) of their pooled practice.
///
/// The drivers are social and psychological. Settled farmers drew together into villages
/// for defence, exchange, marriage partners and shared ritual (Kuijt 2000), so the pull
/// grows with sedentism and draws people to the larger community, never a smaller one;
/// mobile foragers (effort 0) never join. What stops a village growing is the limit of face-to-face coordination
/// (scalar stress, Johnson 1982; the social-brain group size, Dunbar 1993), where fission
/// splits it again (ADR-0070). Joined groups pool their practice, people-weighted.
/// `rate` 0 switches it off.
#[derive(Clone, Debug)]
pub struct Aggregation {
    pub period_ticks: u64,
    pub rate: f64,
    pub fission: Fission,
    pub after: Vec<&'static str>,
}

/// People-weighted mean of a share held by groups of `wb` and `w` people (rounded down).
fn pooled(wb: u64, xb: u32, w: u64, x: u32) -> u32 {
    ((wb as u128 * xb as u128 + w as u128 * x as u128) / (wb + w) as u128) as u32
}

impl Aggregation {
    /// Whether a group of `w` people at cultivation `ca` can join one of `wb` at `cb`: the
    /// joined village stays below the limit of its pooled practice.
    pub fn fits(&self, wb: u64, cb: u32, w: u64, ca: u32) -> bool {
        ((wb + w) as f64) < self.fission.limit(Traits::share(pooled(wb, cb, w, ca)))
    }

    pub fn from_params(p: &Params) -> Aggregation {
        Aggregation {
            period_ticks: p.period_ticks,
            rate: p.aggregation_rate,
            fission: Fission::from_params(p),
            after: vec!["inundation"],
        }
    }
}

impl Mechanism for Aggregation {
    fn spec(&self) -> MechanismSpec {
        MechanismSpec {
            name: "aggregation",
            period_ticks: self.period_ticks,
            reads: vec![Component::AgentTraits],
            writes: vec![Component::AgentWeight, Component::AgentTraits],
            after: self.after.clone(),
        }
    }

    fn decide(&self, sense: &Sense<'_>, part: &mut PartitionView<'_>, tile: Range<usize>) {
        if self.rate == 0.0 {
            return;
        }
        let rng = Philox4x32::new(sense.seed, Stream::Aggregation);
        let t = sense.tick.0;
        let agents = &mut *part.agents;
        if agents.cultivation[tile.clone()].iter().all(|&c| c == 0) {
            return; // foragers' tiles: nobody joins
        }
        // Tiles hold whole cells (agents spawned this step lie past the tile), and groups
        // are taken in canonical order; a cell's range is found only for groups that join.
        for a in tile.clone() {
            let w = agents.weight[a];
            if w == 0 || agents.cultivation[a] == 0 {
                continue;
            }
            let join = Probability::from_f64(self.rate * Traits::share(agents.cultivation[a]));
            if !join.hit(rng.u32(agents.id[a].0, t, 0, 0)) {
                continue;
            }
            let cell = agents.cell[a];
            let first = a - agents.cell[tile.start..a].iter().rev().take_while(|&&c| c == cell).count();
            let end = a + agents.cell[a..tile.end].iter().take_while(|&&c| c == cell).count();
            // The largest other group at least this one's size that has room for it (first
            // in canonical order on ties).
            let mut target: Option<usize> = None;
            for b in first..end {
                let wb = agents.weight[b];
                if b == a || wb < w || target.is_some_and(|t| agents.weight[t] >= wb) {
                    continue;
                }
                if self.fits(wb, agents.cultivation[b], w, agents.cultivation[a]) {
                    target = Some(b);
                }
            }
            let Some(b) = target else { continue };
            let wb = agents.weight[b];
            let (tb, ta) = (agents.traits(b), agents.traits(a));
            agents.set_traits(
                b,
                Traits {
                    cultivation: pooled(wb, tb.cultivation, w, ta.cultivation),
                    domestication: pooled(wb, tb.domestication, w, ta.domestication),
                    inequality: pooled(wb, tb.inequality, w, ta.inequality),
                },
            );
            agents.weight[b] = wb + w;
            agents.weight[a] = 0;
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
///   target, at `adoption_rate`, all of it on its own (individual learning) or, by the
///   `innovation_share` hypothesis, mostly from the people around it: then the gap
///   closes at the rate times the effort it sees, and cultivation spreads as an
///   innovation does, slowly while few plant and then fast (Bass 1969; Henrich 2001;
///   ADR-0065). The target is zero where wild cereals
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
    pub innovation_share: f64,
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
            innovation_share: p.innovation_share,
            selection: p.selection_coefficient,
            wild_frequency: p.domestic_wild_frequency,
            introgression: p.introgression_rate,
            after: vec!["aggregation"],
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

    /// Effort after one period of moving from `effort` toward `target`, for a band that
    /// sees a mean effort of `seen` around it.
    pub fn adopt(&self, effort: f64, target: f64, seen: f64) -> f64 {
        let rate = self.adoption_rate * (self.innovation_share + (1.0 - self.innovation_share) * seen);
        effort + rate * (target - effort)
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
            if target == 0.0 && traits.cultivation == 0 && traits.domestication == 0 {
                continue;
            }
            let (e, d) = (Traits::share(traits.cultivation), Traits::share(traits.domestication));
            let seen = match sense.population[cell] {
                0 => e,
                n => sense.effort_sum[cell] as f64 / (n as f64 * SHARE_ONE as f64),
            };
            agents.set_traits(
                i,
                Traits {
                    cultivation: Traits::toward(traits.cultivation, self.adopt(e, target, seen)),
                    domestication: Traits::toward(traits.domestication, self.domesticate(d, e)),
                    ..traits
                },
            );
        }
    }
}

/// Wealth inequality among a group's households (ADR-0074), as the variance of their log
/// wealth, after the dynamic model of Borgerhoff Mulder et al. (2009): each generation a
/// household keeps a share `beta` of its parents' deviation from the mean (bequests,
/// assortative marriage, returns to wealth) and meets chance shocks of variance `sigma^2`
/// (illness, luck in the hunt or the harvest), so inequality relaxes toward
/// `sigma^2 / (1 - beta^2)`.
///
/// The drivers are social. Where land is open, norms of sharing and levelling keep wealth
/// from being passed on (Woodburn 1982; Boehm 1993): foragers and horticulturalists pass
/// on little (beta 0.19 and 0.18) and are about as equal (Gini 0.25 and 0.27). Where
/// farmland is scarce and closed in, it becomes property and is inherited (Demsetz 1967;
/// Bowles and Choi 2013), and those without it cannot move away from those who claim it
/// (circumscription, Carneiro 1970): agriculturalists pass on more (0.36) and are unequal
/// (0.48). A group's ownership of land is its cultivation effort times how closed in its
/// land is ([`Inequality::closure`]); transmission and shocks move between the two
/// economies with it (linearly, per generation), so farming islands on an open frontier
/// stay equal and inequality rises as farmland fills the landscape.
///
/// Inequality changes nothing else yet: no feedback on fertility, fission, aggregation
/// or conflict. It is updated every period by the equivalent fraction of a generation
/// (tabulated over ownership), so a generation of periods is the generational recursion
/// whatever the period length. Merged groups pool it,
/// people-weighted (the spread between their mean wealth is left out), and split-off
/// groups take their parent's.
#[derive(Clone, Debug)]
pub struct Inequality {
    pub period_ticks: u64,
    /// Per period, at ownership k / [`Inequality::STEPS`]: the share of the variance kept
    /// and the variance added.
    pub per_period: Vec<(f64, f64)>,
    /// The stored inequality of foragers that the update keeps exactly (most agents, most
    /// of the time), so they are skipped; None if rounding has no such fixed point.
    pub forager_fixed: Option<u32>,
    pub after: Vec<&'static str>,
}

impl Inequality {
    /// Ownership steps tabulated: linear interpolation between them errs by under 1e-7.
    pub const STEPS: usize = 1024;

    pub fn from_params(p: &Params) -> Inequality {
        assert!(p.wealth_transmission < 1.0 && p.landed_wealth_transmission < 1.0, "validated parameters");
        let generations =
            p.period_ticks as f64 / (oikos_core::time::TICKS_PER_YEAR as f64 * p.generation_years);
        let per_period = (0..=Self::STEPS)
            .map(|k| {
                let owned = k as f64 / Self::STEPS as f64;
                let beta =
                    p.wealth_transmission + owned * (p.landed_wealth_transmission - p.wealth_transmission);
                let shock = p.wealth_shock + owned * (p.landed_wealth_shock - p.wealth_shock);
                let kept = oikos_math::pow(beta * beta, generations);
                // A generation of periods adds shock: the geometric sum of what each adds.
                (kept, shock * (1.0 - kept) / (1.0 - beta * beta))
            })
            .collect();
        let mut m = Inequality {
            period_ticks: p.period_ticks,
            per_period,
            forager_fixed: None,
            after: vec!["cultivation"],
        };
        let fixed = Self::stored(m.stationary(0.0));
        m.forager_fixed = (m.next_stored(fixed, 0.0) == fixed).then_some(fixed);
        m
    }

    /// Foragers at the inequality they settle at: how new worlds start (ADR-0074).
    pub fn foragers(p: &Params) -> Traits {
        Traits { inequality: Self::stored(Self::from_params(p).stationary(0.0)), ..Traits::FORAGER }
    }

    fn stored(v: f64) -> u32 {
        (oikos_math::finite(v) * SHARE_ONE as f64).round().min(u32::MAX as f64) as u32
    }

    /// (share kept, variance added) per period at ownership `owned` in [0, 1].
    fn coefficients(&self, owned: f64) -> (f64, f64) {
        let x = owned * Self::STEPS as f64;
        let k = (x as usize).min(Self::STEPS);
        if k == Self::STEPS {
            return self.per_period[k];
        }
        let ((a0, b0), (a1, b1), f) = (self.per_period[k], self.per_period[k + 1], x - k as f64);
        (a0 + f * (a1 - a0), b0 + f * (b1 - b0))
    }

    /// [`Inequality::next`] of a stored variance, stored.
    pub fn next_stored(&self, v: u32, owned: f64) -> u32 {
        Self::stored(self.next(v as f64 / SHARE_ONE as f64, owned))
    }

    /// The variance of log wealth a period after `v`, for a group owning its land to the
    /// extent `owned` in [0, 1].
    pub fn next(&self, v: f64, owned: f64) -> f64 {
        let (kept, added) = self.coefficients(owned);
        kept * v + added
    }

    /// The variance of log wealth a group owning its land to the extent `owned` settles at.
    pub fn stationary(&self, owned: f64) -> f64 {
        let (kept, added) = self.coefficients(owned);
        added / (1.0 - kept)
    }

    /// How closed in a cell's land is, from its neighbors as (habitable, people's mean
    /// cultivation effort): the mean of their effort, uninhabitable neighbors (sea,
    /// desert) counting as closed. Open forager country is 0, a farmed landscape 1.
    pub fn closure(neighbors: impl Iterator<Item = (bool, f64)>) -> f64 {
        let (mut sum, mut n) = (0.0, 0u32);
        for (habitable, effort) in neighbors {
            sum += if habitable { effort } else { 1.0 };
            n += 1;
        }
        if n == 0 { 1.0 } else { sum / n as f64 }
    }

    fn closure_of(sense: &Sense<'_>, cell: u32) -> f64 {
        Self::closure(sense.grid.neighbors(cell).iter().map(|&c| {
            let c = c as usize;
            let effort = match sense.population[c] {
                0 => 0.0,
                n => sense.effort_sum[c] as f64 / (n as f64 * SHARE_ONE as f64),
            };
            (sense.habitable[c], effort)
        }))
    }
}

impl Mechanism for Inequality {
    fn spec(&self) -> MechanismSpec {
        MechanismSpec {
            name: "inequality",
            period_ticks: self.period_ticks,
            reads: vec![Component::Population, Component::Forcing, Component::AgentTraits],
            writes: vec![Component::AgentTraits],
            after: self.after.clone(),
        }
    }

    fn decide(&self, sense: &Sense<'_>, part: &mut PartitionView<'_>, tile: Range<usize>) {
        let agents = &mut *part.agents;
        // Foragers at their fixed point stay there; u32::MAX (no fixed point) skips none
        // that the update would change, since stored values only reach it for log
        // variances above 4.29, where it clamps.
        let fixed = self.forager_fixed.unwrap_or(u32::MAX);
        let mut closure: Option<(u32, f64)> = None; // per cell, only where someone farms
        let cultivation = &agents.cultivation[tile.clone()];
        let cells = &agents.cell[tile.clone()];
        for ((&c, &cell), v) in cultivation.iter().zip(cells).zip(&mut agents.inequality[tile]) {
            if c == 0 && *v == fixed {
                continue;
            }
            let owned = if c == 0 {
                0.0
            } else {
                let closed = match closure {
                    Some((at, closed)) if at == cell => closed,
                    _ => Self::closure_of(sense, cell),
                };
                closure = Some((cell, closed));
                Traits::share(c) * closed
            };
            *v = self.next_stored(*v, owned);
        }
    }
}
