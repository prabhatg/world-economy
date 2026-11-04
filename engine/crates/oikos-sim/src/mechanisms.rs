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

use crate::agents::{ALONE, Agents, SHARE_ONE, Settlement, Traits};
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
    /// People fed per cell if all effort went into cultivating fully domestic crops, on
    /// its soil (declare [`Component::Forcing`]).
    pub farm: &'a [f64],
    /// People-weighted sum of effort times domestic share per cell (in
    /// [`crate::agents::SHARE_ONE`] units), after the previous step's commit.
    pub crop_sum: &'a [u64],
    /// Per cell, its people as models of cultivation, (farming, all) as
    /// [`Cultivation::cell_models`] weighs them at `models_for` (success bias, wild yield
    /// ratio), computed once a step for the world's parameters; empty where they learn
    /// from the own cell's effort only ([`Cultivation::own_cell_only`]).
    pub models: &'a [(f64, f64)],
    pub models_for: (f64, f64),
    /// Whether any group lived with others when the step began (ADR-0082); if not, every
    /// group of every tile lives alone until one makes a settlement label in its own tile.
    pub towns: bool,
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
    /// Set when a settlement label is made.
    pub(crate) towns: &'a mut bool,
    /// Daughters that stayed in their settlement this step, past every tile: cell, label,
    /// people and practice.
    pub(crate) stayed: &'a mut Vec<(u32, u32, u64, Traits)>,
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

    /// Creates a group born in `cell` (which this partition owns) that stays there, in its
    /// settlement labeled `settlement`.
    pub fn spawn_in(&mut self, cell: u32, weight: u64, traits: Traits, settlement: u32) -> EntityId {
        let id = self.spawn(cell, cell, weight, traits);
        *self.agents.settlement.last_mut().expect("spawned in this partition") = settlement;
        id
    }

    /// A settlement label no group of `cell` (which this partition owns) holds: one more
    /// than a count drawn from its birth counter, as if a group were born there.
    pub fn new_label(&mut self, cell: u32) -> u32 {
        let counter = &mut self.counters[(cell - self.cells.start) as usize];
        let label = u32::try_from(*counter + 1).expect("fewer than 4 billion groups born in a cell");
        *counter += 1;
        *self.towns = true;
        label
    }

    /// Settlement `s` of `cell` with the daughters that stayed in it this step.
    pub fn with_stayed(&self, cell: u32, mut s: Settlement) -> Settlement {
        if s.label != ALONE {
            let label = s.label;
            for &(_, _, n, t) in self.stayed.iter().filter(|x| x.0 == cell && x.1 == label) {
                s.add_group(n, t);
            }
        }
        s
    }

    /// The label of the settlement group `g` lives in, giving it a new one if it lives alone
    /// (as another group comes to live with it).
    fn label_of(&mut self, g: usize) -> u32 {
        if self.agents.settlement[g] == ALONE {
            self.agents.settlement[g] = self.new_label(self.agents.cell[g]);
        }
        self.agents.settlement[g]
    }
}

/// The run of agents from `i` (to at most `end`) in `i`'s cell.
fn cell_at(agents: &Agents, i: usize, end: usize) -> Range<usize> {
    let cell = agents.cell[i];
    i..i + agents.cell[i..end].iter().take_while(|&&c| c == cell).count()
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
///
/// A settlement of several groups (ADR-0082) shares its water, waste and paths: the
/// infection persists in it as in one group of all its people, at its pooled practice,
/// and each of its groups loses its share of the infected.
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

    /// Expected deaths per period among `w` people of a group in settlement `s` (the
    /// group's own at `effort` if it lives alone).
    fn deaths_in(&self, w: u64, effort: u32, s: &Settlement) -> f64 {
        if s.groups <= 1 {
            return self.deaths(w as f64, Traits::share(effort));
        }
        self.mortality * w as f64 * self.prevalence(s.people as f64, Traits::share(s.cultivation()))
    }

    fn die(rng: &Philox4x32, t: u64, agents: &mut Agents, i: usize, expected: f64) -> u64 {
        let w = agents.weight[i];
        let died = stochastic_round(oikos_math::finite(expected), rng.u32(agents.id[i].0, t, 0, 0)).min(w);
        agents.weight[i] = w - died;
        died
    }
}

impl Mechanism for Disease {
    fn spec(&self) -> MechanismSpec {
        MechanismSpec {
            name: "disease",
            period_ticks: self.period_ticks,
            reads: vec![Component::AgentWeight, Component::AgentTraits, Component::AgentSettlement],
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
        // No group this small carries the infection by itself, whatever its practice: most
        // groups that live alone are skipped.
        let carriers = self.critical_size(1.0) as u64;
        let mut deaths = 0;
        let lone = |agents: &mut Agents, groups: Range<usize>, deaths: &mut u64| {
            for g in groups {
                let w = agents.weight[g];
                if w > carriers {
                    let e = self.deaths(w as f64, Traits::share(agents.cultivation[g]));
                    if e > 0.0 {
                        *deaths += Disease::die(&rng, t, agents, g, e);
                    }
                }
            }
        };
        // Most tiles: every group lives alone.
        if !sense.towns || agents.alone(tile.clone()) {
            lone(agents, tile, &mut deaths);
            part.stats.deaths += deaths;
            return;
        }
        // Settlements of several groups: every group's deaths from the settlement before any.
        let mut expected = Vec::new();
        let mut i = tile.start;
        while i < tile.end {
            let cell = cell_at(agents, i, tile.end);
            i = cell.end;
            if agents.alone(cell.clone()) {
                lone(agents, cell, &mut deaths);
                continue;
            }
            expected.clear();
            for g in cell.clone() {
                let s = agents.settlement_of(cell.clone(), g);
                expected.push(self.deaths_in(agents.weight[g], agents.cultivation[g], &s));
            }
            for (g, &e) in cell.zip(&expected) {
                if e > 0.0 {
                    deaths += Disease::die(&rng, t, agents, g, e);
                }
            }
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
///
/// A settlement may hold several groups (ADR-0082) up to its own limit
/// ([`Fission::settlement_limit`]): `integration` groups' worth, by institutions that bind
/// households across groups without command (ritual, shared buildings, descent; Bandy
/// 2004; Hodder 2014), and `hierarchy` more per unit of its households' variance of log
/// wealth, by leaders whose decisions others accept (Johnson 1982). A daughter that stays
/// in her cell stays in her settlement while it holds fewer people than that, and a
/// settlement at or above it sheds its smallest group, with the same probability, which
/// founds a settlement of its own in the cell: scalar stress between groups, as between
/// households. Integration 1 and no hierarchy are settlements of one group, as before.
#[derive(Clone, Debug)]
pub struct Fission {
    pub period_ticks: u64,
    pub threshold: u64,
    pub village: u64,
    pub probability: f64,
    pub exploration: f64,
    pub integration: f64,
    pub hierarchy: f64,
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
            integration: p.settlement_integration,
            hierarchy: p.settlement_hierarchy,
            after: vec!["disease"],
        }
    }

    /// Size limit of a settlement whose groups' pooled practice is `effort` and pooled
    /// inequality `inequality` (fixed point): `integration` groups' worth, plus `hierarchy`
    /// more per unit of variance of log wealth. A lone group's limit at integration 1 and
    /// no hierarchy.
    pub fn settlement_limit(&self, effort: f64, inequality: u32) -> f64 {
        self.limit(effort) * (self.integration + self.hierarchy * Traits::share(inequality))
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

    /// Whether settlement `s` is at or above its limit.
    fn crowded(&self, s: &Settlement) -> bool {
        s.people as f64 >= self.settlement_limit(Traits::share(s.cultivation()), s.inequality())
    }

    /// Scalar stress between groups: each settlement of several groups in `cell` (one
    /// cell's range) at or above its limit sheds its smallest group (the first on ties),
    /// with probability `split`, which then lives by itself.
    fn shed(&self, sense: &Sense<'_>, part: &mut PartitionView<'_>, cell: Range<usize>, split: Probability) {
        let rng = Philox4x32::new(sense.seed, Stream::Settlement);
        let mut leaving = Vec::new();
        let agents = &*part.agents;
        let here = agents.cell[cell.start];
        agents.for_each_settlement(cell.clone(), |s| {
            if s.groups < 2 || !self.crowded(s) {
                return;
            }
            if !split.hit(rng.u32(EntityId::new(here, s.label as u64).0, sense.tick.0, 0, 0)) {
                return;
            }
            let members =
                (s.first..cell.end).filter(|&g| agents.settlement[g] == s.label && agents.weight[g] > 0);
            leaving.push(members.min_by_key(|&g| agents.weight[g]).expect("a settlement of several groups"));
        });
        for g in leaving {
            part.agents.settlement[g] = ALONE;
        }
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
            writes: vec![Component::AgentWeight, Component::AgentSpawn, Component::AgentSettlement],
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
        // Where every group lives alone (most tiles), each is a settlement of its own: they
        // are taken one by one.
        let alone = !sense.towns || part.agents.alone(tile.clone());
        let mut i = tile.start;
        while i < tile.end {
            if alone && part.agents.weight[i] < smallest {
                i += 1;
                continue;
            }
            let cell_range = if alone { i..i + 1 } else { cell_at(part.agents, i, tile.end) };
            i = cell_range.end;
            let cell = part.agents.cell[cell_range.start];
            // Bands the sea has flooded do not split; inundation moves them this step.
            if !sense.habitable[cell as usize] {
                continue;
            }
            if !alone && !part.agents.alone(cell_range.clone()) {
                self.shed(sense, part, cell_range.clone(), split);
            }
            for g in cell_range.clone() {
                // Weight first: most agents are below every limit, so skip them after one load.
                let mut w = part.agents.weight[g];
                if w < smallest || !self.splits(w, Traits::share(part.agents.cultivation[g])) {
                    continue;
                }
                let id = part.agents.id[g];
                if !split.hit(fission_rng.u32(id.0, t, 0, 0)) {
                    continue;
                }
                let c = cell as usize;
                let room = self.village != 0 && sense.population[c] < sense.capacity[c];
                let dest =
                    if room { cell } else { self.destination(sense, &migration_rng, explore, id, cell) };
                // A daughter who stays in her cell stays in her settlement while it has room.
                let stays = self.village != 0
                    && dest == cell
                    && !self
                        .crowded(&part.with_stayed(cell, part.agents.settlement_of(cell_range.clone(), g)));
                let take = w / 2;
                let half = Agents::split_weight(&mut w, take);
                part.agents.weight[g] = w;
                part.stats.fissions += 1;
                let traits = part.agents.traits(g);
                if stays {
                    let label = part.label_of(g);
                    part.spawn_in(cell, half, traits, label);
                    part.stayed.push((cell, label, half, traits));
                } else {
                    part.spawn(cell, dest, half, traits);
                }
            }
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
///
/// Settlements may hold several groups (ADR-0082): a group joins the largest other
/// settlement of its cell at least as large as its own that has room for it, merging into
/// the largest of its groups at least its own size that it fits (as above), and otherwise
/// living beside them as a group of its own, if the settlement stays below its limit
/// ([`Fission::settlement_limit`]) with it. Settlements of one group are groups, as before.
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

    /// Whether a group of `w` people with practice `t` can join settlement `s` as a group
    /// of its own: the settlement stays below the limit of its pooled practice.
    pub fn settles(&self, s: &Settlement, w: u64, t: Traits) -> bool {
        let (cultivation, inequality) = s.joined(w, t);
        ((s.people + w) as f64) < self.fission.settlement_limit(Traits::share(cultivation), inequality)
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
            writes: vec![Component::AgentWeight, Component::AgentTraits, Component::AgentSettlement],
            after: self.after.clone(),
        }
    }

    fn decide(&self, sense: &Sense<'_>, part: &mut PartitionView<'_>, tile: Range<usize>) {
        if self.rate == 0.0 {
            return;
        }
        let rng = Philox4x32::new(sense.seed, Stream::Aggregation);
        let t = sense.tick.0;
        if part.agents.cultivation[tile.clone()].iter().all(|&c| c == 0) {
            return; // foragers' tiles: nobody joins
        }
        // Tiles hold whole cells (agents spawned this step lie past the tile), and groups
        // are taken in canonical order; a cell's range is found only for groups that join.
        for a in tile.clone() {
            let agents = &*part.agents;
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
            let ta = agents.traits(a);
            let own = part.with_stayed(cell, agents.settlement_of(first..end, a));
            // The largest other settlement at least as large as this group's own that has room
            // for it (first in canonical order on ties), and the group of it to merge into: the
            // largest at least this one's size that it fits (first on ties), if any.
            let mut target: Option<(Settlement, Option<usize>)> = None;
            agents.for_each_settlement(first..end, |s| {
                let s = &part.with_stayed(cell, *s);
                if agents.lives_in(a, s)
                    || s.people < own.people
                    || target.is_some_and(|(t, _)| t.people >= s.people)
                {
                    return;
                }
                let mut group: Option<usize> = None;
                for b in s.first..end {
                    let wb = agents.weight[b];
                    if !agents.lives_in(b, s) || wb < w || group.is_some_and(|g| agents.weight[g] >= wb) {
                        continue;
                    }
                    if self.fits(wb, agents.cultivation[b], w, ta.cultivation) {
                        group = Some(b);
                    }
                }
                if group.is_some() || self.settles(s, w, ta) {
                    target = Some((*s, group));
                }
            });
            let Some((s, group)) = target else { continue };
            let Some(b) = group else {
                part.agents.settlement[a] = part.label_of(s.first);
                continue;
            };
            let agents = &mut *part.agents;
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
///   ADR-0065). What it sees is the share of cultivation among the people of its cell
///   and, by `neighbor_learning`, of the cells around it, which kinship, marriage and
///   visiting link it to (Wobst 1974); and people copy those who visibly do well, so by
///   `success_bias` each farmer counts as farming's payoff, what a unit of effort feeds
///   against foraging, to that power (payoff bias: Boyd and Richerson 1985; McElreath
///   et al. 2008; ADR-0076). Farming that pays less than foraging, as early
///   cultivation of wild cereals can, is then copied less; and a band copies the
///   practice it moves toward, foragers' when it gives cultivation up. The target is zero where wild cereals
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
    pub neighbor_learning: f64,
    pub success_bias: f64,
    pub wild_yield_ratio: f64,
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
            neighbor_learning: p.neighbor_learning,
            success_bias: p.success_bias,
            wild_yield_ratio: p.wild_yield_ratio,
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
    /// sees a share `seen` of cultivation around it. It copies the practice it moves
    /// toward (ADR-0076): farmers' when taking cultivation up, foragers' when giving it
    /// up. Learning from the own cell by how many plant (ADR-0065) paces both by farmers.
    pub fn adopt(&self, effort: f64, target: f64, seen: f64) -> f64 {
        let models = if target < effort && !self.own_cell_only() { 1.0 - seen } else { seen };
        let rate = self.adoption_rate * (self.innovation_share + (1.0 - self.innovation_share) * models);
        effort + rate * (target - effort)
    }

    /// Farming's payoff in a cell: the people a unit of effort feeds cultivating against
    /// those it feeds foraging ([`crate::environment::farmed_capacity`]), from the cell's
    /// full-farming capacity `farm`, its forager capacity and its people's sums of effort
    /// and of effort times domestic share.
    pub fn payoff(&self, farm: f64, forage: u64, effort_sum: u64, crop_sum: u64) -> f64 {
        let domestic = if effort_sum == 0 { 0.0 } else { crop_sum as f64 / effort_sum as f64 };
        let w = self.wild_yield_ratio;
        oikos_math::finite(farm * (w + (1.0 - w) * domestic) / forage.max(1) as f64)
    }

    /// Whether bands learn from their own cell only, by how many plant (ADR-0065).
    pub fn own_cell_only(&self) -> bool {
        self.neighbor_learning == 0.0 && self.success_bias == 0.0
    }

    /// `people` at mean effort `effort` as models of cultivation, (farming, all): each
    /// person's farming effort counts farming's `payoff` to the power `success_bias`, and
    /// the rest of their effort, foraging, counts 1.
    pub fn models(&self, people: u64, effort: f64, payoff: f64) -> (f64, f64) {
        let n = people as f64;
        let farming = match self.success_bias {
            _ if effort == 0.0 => 0.0,
            0.0 => effort,
            1.0 => effort * payoff, // linear payoff weighting: no pow per cell and step
            bias => effort * oikos_math::pow(payoff, bias),
        };
        // The foraging share first: farming + 1 - effort cancels when farming is tiny.
        (n * farming, n * (farming + (1.0 - effort)))
    }

    /// [`Cultivation::models`] for a cell's people, from its full-farming capacity, forager
    /// capacity and its people's sums of effort and of effort times domestic share.
    pub fn cell_models(
        &self,
        farm: f64,
        forage: u64,
        people: u64,
        effort_sum: u64,
        crop_sum: u64,
    ) -> (f64, f64) {
        let effort = if people == 0 { 0.0 } else { effort_sum as f64 / (people as f64 * SHARE_ONE as f64) };
        let payoff = if effort_sum > 0 && self.success_bias != 0.0 {
            self.payoff(farm, forage, effort_sum, crop_sum)
        } else {
            1.0
        };
        self.models(people, effort, payoff)
    }

    /// The share of cultivation in what a band learns from, given its models as (contact,
    /// people, their mean effort, farming's payoff there): each person counts `contact`
    /// times [`Cultivation::models`]. None if it sees nobody.
    pub fn example(&self, models: impl Iterator<Item = (f64, u64, f64, f64)>) -> Option<f64> {
        Self::example_of(models.map(|(contact, n, effort, payoff)| (contact, self.models(n, effort, payoff))))
    }

    fn example_of(models: impl Iterator<Item = (f64, (f64, f64))>) -> Option<f64> {
        let (mut farming, mut all) = (0.0, 0.0);
        for (contact, (f, a)) in models {
            if contact != 0.0 {
                farming += contact * f;
                all += contact * a;
            }
        }
        (all > 0.0).then(|| oikos_math::finite(farming / all))
    }

    /// [`Cultivation::example`] for a band in `cell`: its cell and its neighbors.
    fn example_at(&self, sense: &Sense<'_>, cell: u32) -> Option<f64> {
        let cached = sense.models.len() == sense.population.len()
            && sense.models_for == (self.success_bias, self.wild_yield_ratio);
        let at = |c: u32, contact: f64| {
            let c = c as usize;
            let models = if cached {
                sense.models[c]
            } else {
                let (farm, forage) = (sense.farm[c], sense.forage_capacity[c]);
                self.cell_models(farm, forage, sense.population[c], sense.effort_sum[c], sense.crop_sum[c])
            };
            (contact, models)
        };
        let neighbors = sense.grid.neighbors(cell).iter().map(|&c| at(c, self.neighbor_learning));
        Self::example_of(std::iter::once(at(cell, 1.0)).chain(neighbors))
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
        // Learning from the own cell by how many plant (ADR-0065) is computed as it was.
        let own_cell_only = self.own_cell_only();
        let mut example: Option<(usize, Option<f64>)> = None; // per cell, only where needed
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
            let seen = if own_cell_only {
                match sense.population[cell] {
                    0 => e,
                    n => sense.effort_sum[cell] as f64 / (n as f64 * SHARE_ONE as f64),
                }
            } else {
                let seen = match example {
                    Some((at, seen)) if at == cell => seen,
                    _ => self.example_at(sense, cell as u32),
                };
                example = Some((cell, seen));
                seen.unwrap_or(e)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inputs::Inputs;

    const ONE: u32 = SHARE_ONE;

    fn id(counter: u64) -> EntityId {
        EntityId::new(0, counter)
    }

    /// Settlement scenes: the village limit is 127 for full farmers, and they always join.
    fn params(integration: f64) -> Params {
        Params {
            fission_threshold: 60,
            village_fission_size: 127,
            settlement_integration: integration,
            settlement_hierarchy: 0.0,
            aggregation_rate: 1.0,
            fission_probability: 1.0,
            exploration_probability: 0.0,
            ..Params::default()
        }
    }

    /// Full farmers of cell 0: (counter, people, settlement label).
    fn farmers(groups: &[(u64, u64, u32)]) -> Agents {
        let mut a = Agents::default();
        let t = Traits { cultivation: ONE, ..Traits::FORAGER };
        for &(c, w, s) in groups {
            a.push_in(id(c), 0, w, t, s);
        }
        a
    }

    /// Whether groups `g` all live in one settlement of several groups.
    fn together(a: &Agents, g: &[usize]) -> bool {
        a.settlement[g[0]] != ALONE && g.iter().all(|&i| a.settlement[i] == a.settlement[g[0]])
    }

    /// Runs `m` once over one partition of a small grid whose cells hold `capacity`
    /// people; returns the agents, with any spawned this step at the end. Cell 0's birth
    /// counter starts at 100.
    fn run(m: &dyn Mechanism, agents: Agents, capacity: u64) -> Agents {
        run_all(&[m], agents, capacity)
    }

    /// [`run`] for several mechanisms in turn, as one step runs them.
    fn run_all(ms: &[&dyn Mechanism], mut agents: Agents, capacity: u64) -> Agents {
        let grid = Grid::from_base_cell(22, 1).unwrap();
        let n = grid.len();
        let inputs = Inputs::none();
        let mut population = vec![0; n];
        for (&c, &w) in agents.cell.iter().zip(&agents.weight) {
            population[c as usize] += w;
        }
        let (habitable, capacity, zeros) = (vec![true; n], vec![capacity; n], vec![0; n]);
        let sense = Sense {
            grid: &grid,
            inputs: &inputs,
            habitable: &habitable,
            restricted: false,
            capacity: &capacity,
            forage_capacity: &capacity,
            farming: false,
            suitability: &[],
            climate_stress: &[],
            climate_variability: &[],
            population: &population,
            effort_sum: &zeros,
            farm: &[],
            crop_sum: &zeros,
            models: &[],
            models_for: (0.0, 0.0),
            towns: true,
            tick: Tick(1),
            seed: 7,
        };
        let (mut stats, mut counters) = (StepStats::default(), vec![100; n]);
        let mut outbox = vec![Agents::default()];
        let ranges = std::slice::from_ref(&(0..n as u32)).to_vec();
        let (len, mut towns, mut stayed) = (agents.len(), false, Vec::new());
        let mut part = PartitionView {
            cells: 0..n as u32,
            agents: &mut agents,
            stats: &mut stats,
            counters: &mut counters,
            outbox: &mut outbox,
            ranges: &ranges,
            towns: &mut towns,
            stayed: &mut stayed,
        };
        for m in ms {
            m.decide(&sense, &mut part, 0..len);
        }
        agents
    }

    #[test]
    fn a_group_too_large_to_merge_joins_a_settlement_as_a_group_of_its_own() {
        // 90 people cannot merge into a group of 100 (127 is the limit of a group), but a
        // settlement holding three groups' worth takes them as a neighbourhood of their own.
        let a = run(&Aggregation::from_params(&params(3.0)), farmers(&[(1, 100, 0), (2, 90, 0)]), 1000);
        assert_eq!(a.weight, [100, 90], "both groups keep their people");
        assert!(together(&a, &[0, 1]), "{:?}", a.settlement);
        assert_eq!(a.settlement[0], 101, "a label new to the cell, from its birth counter");
    }

    #[test]
    fn a_group_that_fits_still_merges_into_the_largest_group_it_fits() {
        for integration in [1.0, 3.0] {
            let scene = farmers(&[(1, 100, 0), (2, 90, 0), (3, 20, 0)]);
            let a = run(&Aggregation::from_params(&params(integration)), scene, 1000);
            assert_eq!(a.weight, [120, 90, 0], "integration {integration}");
            let towns = integration > 1.0;
            assert_eq!(together(&a, &[0, 1]), towns, "integration {integration}: {:?}", a.settlement);
            assert!(towns || a.alone(0..3), "a settlement of one group is a group");
        }
    }

    #[test]
    fn a_full_settlement_takes_no_more_groups() {
        // Two groups' worth, 254 people: 190 and 80 more is too many.
        let a = run(
            &Aggregation::from_params(&params(2.0)),
            farmers(&[(1, 100, 5), (2, 90, 5), (4, 80, 0)]),
            1000,
        );
        assert_eq!(a.weight, [100, 90, 80]);
        assert_eq!(a.settlement, [5, 5, ALONE]);
    }

    #[test]
    fn hierarchy_lets_unequal_settlements_hold_more_groups() {
        let p = Params { settlement_hierarchy: 1.5, ..params(2.0) };
        let f = Fission::from_params(&p);
        assert_eq!(f.settlement_limit(1.0, 0), 254.0, "two groups' worth where wealth is equal");
        assert!((f.settlement_limit(1.0, Traits::from_share(0.4)) - 127.0 * 2.6).abs() < 1e-9);
        assert_eq!(Fission::from_params(&params(1.0)).settlement_limit(0.5, 0), f.limit(0.5));
    }

    #[test]
    fn a_daughter_stays_in_her_settlement_while_it_has_room() {
        // A group of 130 is above its limit and splits; its land has room, so the daughter
        // stays in the cell, and in the settlement if it holds fewer than its limit.
        for (integration, stays) in [(3.0, true), (1.0, false)] {
            let a = run(&Fission::from_params(&params(integration)), farmers(&[(1, 130, 0)]), 1000);
            assert_eq!(a.weight, [65, 65]);
            assert_eq!(a.cell, [0, 0]);
            assert_eq!(together(&a, &[0, 1]), stays, "integration {integration}: {:?}", a.settlement);
            assert!(stays || a.alone(0..2));
        }
    }

    #[test]
    fn a_settlement_counts_the_daughters_that_stayed_this_step() {
        // 130 + 100 people in a settlement of two groups' worth (254): the 130 split, and
        // the daughter stays (230 < 254). Joining 80 more would make 310, so they may not,
        // though the daughter is not one of the step's tile.
        let (fission, aggregation) =
            (Fission::from_params(&params(2.0)), Aggregation::from_params(&params(2.0)));
        let a = run_all(&[&fission, &aggregation], farmers(&[(1, 130, 5), (2, 100, 5), (3, 80, 0)]), 1000);
        assert_eq!(a.weight, [65, 100, 80, 65]);
        assert_eq!(a.settlement, [5, 5, ALONE, 5]);
    }

    #[test]
    fn a_daughter_put_back_in_her_full_cell_stays_in_her_settlement() {
        // The cell is full, so the daughter is placed by crowding, and its neighbors are
        // fuller still: she stays in the cell, and the settlement (limit 381) has room.
        let mut scene = farmers(&[(1, 130, 0)]);
        for c in 1..7 {
            for k in 1..4 {
                scene.push(EntityId::new(c, k), c, 50); // foragers' camps below their limit
            }
        }
        let a = run(&Fission::from_params(&params(3.0)), scene, 100);
        let home: Vec<usize> = (0..a.len()).filter(|&i| a.cell[i] == 0).collect();
        assert_eq!(home.iter().map(|&i| a.weight[i]).collect::<Vec<_>>(), [65, 65]);
        assert!(together(&a, &home), "{:?}", a.settlement);
    }

    #[test]
    fn a_settlement_above_its_limit_sheds_its_smallest_group() {
        // 270 people where two groups' worth (254) is the limit: the smallest group leaves
        // to live by itself (scalar stress between groups).
        let a =
            run(&Fission::from_params(&params(2.0)), farmers(&[(1, 120, 5), (2, 100, 5), (3, 50, 5)]), 1000);
        assert_eq!(a.weight, [120, 100, 50], "no group is above its own limit");
        assert_eq!(a.settlement, [5, 5, ALONE]);
    }

    #[test]
    fn a_group_shares_the_infection_of_its_settlement() {
        // Two villages of 100, critical size 50: alone, each keeps 50 infected; as one
        // settlement of 200, three quarters of each are (prevalence 1 - 50/200).
        let p = Params { disease_threshold: 50.0, disease_mortality: 1.0, ..params(2.0) };
        let alone = run(&Disease::from_params(&p), farmers(&[(1, 100, 0), (2, 100, 0)]), 1000);
        assert_eq!(alone.weight, [50, 50]);
        let together = run(&Disease::from_params(&p), farmers(&[(1, 100, 3), (2, 100, 3)]), 1000);
        assert_eq!(together.weight, [25, 25]);
    }
}
