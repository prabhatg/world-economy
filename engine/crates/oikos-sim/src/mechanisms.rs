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

use crate::agents::{ALONE, Agents, EINKORN, EMMER, SHARE_ONE, Settlement, Traits};
use crate::catchment::Catchments;
use crate::params::Params;
use crate::schedule::{Component, MechanismSpec};
use crate::world::StepStats;
use oikos_core::prob::stochastic_round;
use oikos_core::time::TICKS_PER_YEAR;
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
///
/// Where catchments reach beyond cells (#50, [`crate::catchment`]), the people per cell,
/// their practice sums and what the land feeds (`population`, the `_sum` fields,
/// `capacity`, `forage_capacity`, `farm`, `flood`, `range_feeds`, `pasture_feeds`) are
/// counted over each cell's catchment: its own and its share of each cell around it.
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
    /// Year-to-year spread of each cell's harvest for a band cultivating fully (#13;
    /// declare [`Component::Forcing`]); empty without a climate pack.
    pub harvest_cv: &'a [f64],
    /// Population per cell after the previous step's commit.
    pub population: &'a [u64],
    /// People-weighted sum of the agents' cultivation effort per cell (in
    /// [`crate::agents::SHARE_ONE`] units), after the previous step's commit.
    pub effort_sum: &'a [u64],
    /// People fed per cell if all effort went into cultivating fully domestic crops, on
    /// its soil (declare [`Component::Forcing`]).
    pub farm: &'a [f64],
    /// Floodplain fields per cell beyond what rain grows, as a share of its dry land at
    /// full yield, sown only with domestic crops (ADR-0085).
    pub floodplain: &'a [f64],
    /// People fed per cell if all effort went into domestic crops on its floodplain.
    pub flood: &'a [f64],
    /// Range of wild sheep and goats, and pasture for domestic ones, per cell as shares of
    /// its dry land (#37).
    pub range: &'a [f64],
    pub pasture: &'a [f64],
    /// People fed per cell if all effort went into herding domestic sheep and goats on the
    /// wild herds' range, and on all its pasture.
    pub range_feeds: &'a [f64],
    pub pasture_feeds: &'a [f64],
    /// People-weighted sums of herding effort, and of it times the herd's domestic share,
    /// per cell (in [`crate::agents::SHARE_ONE`] units), after the previous step's commit.
    pub herd_sum: &'a [u64],
    pub herd_crop_sum: &'a [u64],
    /// People-weighted sum of effort times domestic share per cell (in
    /// [`crate::agents::SHARE_ONE`] units), after the previous step's commit.
    pub crop_sum: &'a [u64],
    /// Likewise for emmer and einkorn (#38), sown with the crop.
    pub wheat_sum: &'a [[u64; 2]],
    /// Per cell, its people as models of cultivation, (farming, all) as
    /// [`Cultivation::cell_models`] weighs them at `models_for` (success bias, wild yield
    /// ratio), computed once a step for the world's parameters; empty where they learn
    /// from the own cell's effort only ([`Cultivation::own_cell_only`]).
    pub models: &'a [(f64, f64)],
    pub models_for: (f64, f64),
    /// Each cell's catchment (#50).
    pub catchments: &'a Catchments,
    /// Each cell's own people after the previous step's commit (`population` where every
    /// cell is its own catchment).
    pub cell_population: &'a [u64],
    /// Each cell's largest settlement after the previous step's commit, where catchments
    /// reach beyond cells (empty where every cell is its own).
    pub largest: &'a [Settlement],
    /// Whether any group lived with others when the step began (ADR-0082); if not, every
    /// group of every tile lives alone until one makes a settlement label in its own tile.
    pub towns: bool,
    pub tick: Tick,
    pub seed: u64,
}

impl Sense<'_> {
    /// Carrying capacity of `cell` once `weight` people of practice `t` have joined its
    /// people (#45): the land is shared, so newcomers and residents pool their practice,
    /// people-weighted, as the cell's capacity will once they live there, at the yields
    /// `(wild_yield_ratio, wild_herd_ratio)`. Without farming, its capacity.
    pub fn capacity_joined(&self, cell: u32, weight: u64, t: &Traits, yields: (f64, f64)) -> u64 {
        let c = cell as usize;
        if !self.farming {
            return self.capacity[c];
        }
        // As the cell's sums will be once the group lives there.
        let (w, people) = (weight as f64, (self.population[c] as f64 + weight as f64) * SHARE_ONE as f64);
        let [e, d, h, hd] = t.practice();
        let share = |sum: u64, x: u64| (sum as f64 + w * x as f64) / people;
        let practice = crate::environment::Practice {
            effort: share(self.effort_sum[c], e),
            crop: share(self.crop_sum[c], d),
            herding: share(self.herd_sum[c], h),
            herd_crop: share(self.herd_crop_sum[c], hd),
        };
        let land = crate::environment::Land {
            forage: self.forage_capacity[c],
            farm: self.farm[c],
            flood: self.flood[c],
            herd_range: self.range_feeds[c],
            herd_pasture: self.pasture_feeds[c],
        };
        crate::environment::capacity_at(&land, &practice, yields)
    }
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
        self.spawn_settled(birth_cell, dest, weight, traits, ALONE)
    }

    /// [`PartitionView::spawn`] into the settlement labeled `settlement` of `dest` (a group
    /// moving to a village of another cell, #50; [`ALONE`] to live by itself).
    pub fn spawn_settled(
        &mut self,
        birth_cell: u32,
        dest: u32,
        weight: u64,
        traits: Traits,
        settlement: u32,
    ) -> EntityId {
        let counter = &mut self.counters[(birth_cell - self.cells.start) as usize];
        let id = EntityId::new(birth_cell, *counter);
        *counter += 1;
        if dest != birth_cell {
            self.stats.migrants += weight;
        }
        if self.cells.contains(&dest) {
            self.agents.push_in(id, dest, weight, traits, settlement);
        } else {
            let owner = self.ranges.partition_point(|r| r.end <= dest);
            self.outbox[owner].push_in(id, dest, weight, traits, settlement);
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
    least_crowded_by(sense, cells, |c| sense.capacity[c as usize]).map(|(c, _)| c)
}

/// [`least_crowded`] by the capacities `capacity(c)`, with the chosen cell's capacity.
fn least_crowded_by(
    sense: &Sense<'_>,
    cells: impl IntoIterator<Item = u32>,
    capacity: impl Fn(u32) -> u64,
) -> Option<(u32, u64)> {
    least_crowded_among(cells.into_iter().map(|c| (c, sense.population[c as usize], capacity(c))), false)
}

/// The least crowded of `(cell, people, capacity)` by N/K (the first on ties), and its
/// capacity; `feeding` leaves out places that feed nobody, which compare as no more
/// crowded than any other and, empty, as no less (#13).
fn least_crowded_among(
    cells: impl IntoIterator<Item = (u32, u64, u64)>,
    feeding: bool,
) -> Option<(u32, u64)> {
    let mut best: Option<(u32, u128, u128)> = None;
    for (c, n, k) in cells {
        if feeding && k == 0 {
            continue;
        }
        let (n, k) = (n as u128, k as u128);
        if best.is_none_or(|(_, bn, bk)| n * bk < bn * k) {
            best = Some((c, n, k));
        }
    }
    best.map(|(c, _, k)| (c, k as u64))
}

fn habitable_neighbors<'a>(sense: &'a Sense<'_>, cell: u32) -> impl Iterator<Item = u32> + Clone + 'a {
    sense.grid.neighbors(cell).iter().copied().filter(|&c| sense.habitable[c as usize])
}

/// The habitable cells a group leaving `cell` may found its new home in, beyond its
/// catchment ([`Catchments::away`], #50): its neighbors where every cell is its own.
fn habitable_away<'a>(sense: &'a Sense<'_>, cell: u32) -> impl Iterator<Item = u32> + Clone + 'a {
    sense.catchments.away(sense.grid, cell as usize).iter().copied().filter(|&c| sense.habitable[c as usize])
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
///
/// Years differ (#13): the capacity a band lives on this year is its cell's times
/// 1 + its harvest's spread times the cell's shock ([`harvest_shock`]), at least 0. The
/// spread is the cell's for a band cultivating fully (`Sense::harvest_cv`) times a share
/// that moves linearly with its cultivation effort from `forage_risk_ratio` to 1:
/// foragers and herders, with many wild foods and free to move, feel less of a bad year
/// than farmers tied to their fields (Winterhalder 1986; Halstead and O'Shea 1989). A
/// bad year kills at most the people its harvest cannot feed (or births' worth, if
/// more): the logistic would kill everyone in a year that feeds nobody, and beyond.
///
/// Groups store food (#13; [`Growth::stored`]): in a year whose harvest feeds more than
/// the cell's people a group puts the surplus by, up to `storage_years` of its people's
/// need times its cultivation effort, and in one that feeds fewer it eats from its store;
/// a share `storage_loss` of the store is lost each year. Storing takes a delayed-return
/// economy and norms that let a household keep its store against demands to share
/// (Woodburn 1982; Testart 1982), which settled cultivators have, as their granaries and
/// household stores show (Kuijt and Finlayson 2009; Bogaard et al. 2009), and mobile
/// foragers mostly do not.
///
/// Groups share (#13; [`pooled_harvest`]): a group pools `sharing` of its year's harvest
/// with the people of its neighboring cells (each cell's own people and harvest, whatever
/// the catchments), living on the rest of its own and that share of the mean of theirs at
/// its own practice, so it draws on neighbors the year favoured and gives in turn when
/// they are short. Among foragers, partners in other camps share food and access to land
/// in bad years (Wiessner 1982); among early farmers, households and villages exchange
/// food and obligations against future need (Halstead 1989); transfers that follow need
/// pool risk best where harvests are volatile (Aktipis et al. 2016). The pooling is in
/// expectation: what a group draws in a year is not taken from a neighbor's people.
/// People with buffers of their own lean on reciprocity less (Cashdan 1985): a group
/// withholds `sharing_withdrawal` of its sharing as far as its store and herds insure it
/// ([`Growth::sharing_of`]), and household stores were kept from demands to share
/// (Testart 1982; Bogaard et al. 2009).
#[derive(Clone, Debug)]
pub struct Growth {
    pub period_ticks: u64,
    pub rate: f64,
    pub sedentary_rate: f64,
    pub fertility: f64,
    pub sedentary_fertility: f64,
    pub forage_risk_ratio: f64,
    pub harvest_correlation: f64,
    pub harvest_persistence: f64,
    pub storage_years: f64,
    pub storage_loss: f64,
    pub sharing: f64,
    pub sharing_withdrawal: f64,
    /// Years per period, and the share of a store kept over one, at `storage_loss` a year.
    pub period_years: f64,
    pub store_kept: f64,
}

/// A cell's harvest shock in simulated year `year` (#13): standard (mean 0, variance 1),
/// the square root of `correlation` times the region's shock that year plus the square
/// root of the rest times the cell's own, each a sum of four uniform draws scaled to unit
/// variance (Irwin-Hall), so a drought strikes many cells at once.
///
/// Dry years come in runs (open question 44): with `persistence` above 0 the shock is a
/// first-order autoregression of those yearly draws, the sum of each past year's times
/// `persistence` to the power of its age, scaled back to unit variance and cut where the
/// weights' squares fall below [`PERSISTENCE_TAIL`], so a year carries `persistence` of
/// its shock into the next. Years before the run's start are drawn like any other.
pub fn harvest_shock(seed: u64, cell: u32, year: u64, correlation: f64, persistence: f64) -> f64 {
    let rng = Philox4x32::new(seed, Stream::Harvest);
    let standard = |subject: u64, year: u64| {
        let sum: f64 = rng.block(subject, year, 0).iter().map(|&u| (u as f64 + 0.5) / 4_294_967_296.0).sum();
        (sum - 2.0) * 3f64.sqrt()
    };
    let drawn = |year: u64| {
        let own = standard(cell as u64, year);
        if correlation <= 0.0 {
            return own;
        }
        correlation.sqrt() * standard(u64::MAX, year) + (1.0 - correlation).sqrt() * own
    };
    if persistence <= 0.0 {
        return drawn(year);
    }
    let (mut sum, mut norm, mut weight, mut age) = (0.0, 0.0, 1.0, 0u64);
    while weight * weight >= PERSISTENCE_TAIL {
        sum += weight * drawn(year.wrapping_sub(age));
        norm += weight * weight;
        weight *= persistence;
        age += 1;
    }
    sum / norm.sqrt()
}

/// Squared weight below which a past year's draw is left out of a persistent shock: the
/// years left out carry under a millionth of its variance.
pub const PERSISTENCE_TAIL: f64 = 1e-6;

/// The share of its cell's capacity a band at `effort` lives on in a year of `shock`, its
/// cell's harvest spreading by `cell_cv` for a band cultivating fully: 1 + its spread,
/// moving with its effort from `forage_risk_ratio` of the cell's to all of it, times the
/// shock, at least 0 (#13).
pub fn harvest_share(forage_risk_ratio: f64, effort: f64, cell_cv: f64, shock: f64) -> f64 {
    let cv = cell_cv * (forage_risk_ratio + (1.0 - forage_risk_ratio) * effort);
    (1.0 + cv * shock).max(0.0)
}

/// The share of its capacity a group lives on when it pools `sharing` of its year's
/// harvest `own` with its neighbors', whose shares at its practice are `neighbors` (#13):
/// the rest of its own and `sharing` of their mean; its own where nobody is near.
pub fn pooled_harvest(own: f64, neighbors: &[f64], sharing: f64) -> f64 {
    if sharing == 0.0 || neighbors.is_empty() {
        return own;
    }
    let mean = neighbors.iter().sum::<f64>() / neighbors.len() as f64;
    // Exactly its own where the neighbors' years are its own.
    own + sharing * (mean - own)
}

/// How full a store of `store` (years of need, fixed point) is, against the
/// `storage_years` a fully cultivating group can keep: at most 1, and 0 where nobody
/// stores.
pub fn store_fill(store: u32, storage_years: f64) -> f64 {
    if storage_years > 0.0 { (Traits::share(store) / storage_years).min(1.0) } else { 0.0 }
}

/// [`harvest_shock`] in the year of `tick`: one harvest a year, however often growth runs.
pub fn harvest_at(seed: u64, cell: u32, tick: Tick, correlation: f64, persistence: f64) -> f64 {
    harvest_shock(seed, cell, tick.years(), correlation, persistence)
}

impl Growth {
    pub fn from_params(p: &Params) -> Growth {
        Growth {
            period_ticks: p.period_ticks,
            rate: p.growth_rate,
            sedentary_rate: p.sedentary_growth_rate,
            fertility: p.fertility,
            sedentary_fertility: p.sedentary_fertility,
            forage_risk_ratio: p.forage_risk_ratio,
            harvest_correlation: p.harvest_correlation,
            harvest_persistence: p.harvest_persistence,
            storage_years: p.storage_years,
            storage_loss: p.storage_loss,
            sharing: p.harvest_sharing,
            sharing_withdrawal: p.sharing_withdrawal,
            period_years: p.period_ticks as f64 / TICKS_PER_YEAR as f64,
            // Exact at the yearly period; compounded over shorter or longer ones.
            store_kept: if p.period_ticks == TICKS_PER_YEAR {
                1.0 - p.storage_loss
            } else {
                oikos_math::pow(1.0 - p.storage_loss, p.period_ticks as f64 / TICKS_PER_YEAR as f64)
            },
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
    /// people and capacity `k`: deaths at most the share of the people `k` does not feed,
    /// or births' worth if more.
    pub fn vital_rates(&self, effort: f64, pop: f64, k: f64) -> (f64, f64) {
        let b = self.fertility(effort);
        if k <= 0.0 {
            return (b, b.max(1.0));
        }
        let d = (b - self.rate(effort) * (1.0 - pop / k)).max(0.0);
        (b, if pop > k { d.min((1.0 - k / pop).max(b)) } else { d })
    }

    /// A period of a group at `effort` holding `store` years of its people's need, in a
    /// year whose harvest feeds `fed` times its cell's people: what it eats, as a multiple
    /// of their need, and its store after the period's losses. A shortfall is made up from
    /// the store while it lasts; a surplus goes into it up to `storage_years` times
    /// `effort`; both at their yearly pace over the period.
    pub fn stored(&self, effort: f64, store: f64, fed: f64) -> (f64, f64) {
        let y = self.period_years;
        let (eaten, kept) = if fed < 1.0 {
            let draw = store.min((1.0 - fed) * y);
            (fed + draw / y, store - draw)
        } else {
            let room = self.storage_years * effort - store;
            (fed, if room > 0.0 { store + room.min((fed - 1.0) * y) } else { store })
        };
        (eaten, oikos_math::finite(kept * self.store_kept).max(0.0))
    }

    /// The share of its harvest a group holding `store` (years of need, fixed point) and
    /// spending `herding` of its effort herding pools with its neighbors: `sharing`, less
    /// `sharing_withdrawal` of it times how far store and herds insure it together. A
    /// store of `storage_years` insures fully, as does herding with all its effort.
    pub fn sharing_of(&self, store: u32, herding: u32) -> f64 {
        if self.sharing_withdrawal == 0.0 || (store == 0 && herding == 0) {
            return self.sharing;
        }
        let insured = 1.0 - (1.0 - store_fill(store, self.storage_years)) * (1.0 - Traits::share(herding));
        self.sharing * (1.0 - self.sharing_withdrawal * insured)
    }

    /// A store of `store` (fixed point, per person) held by `w` people, shared among the
    /// `w_new` they become: births share it and the dead leave theirs (rounded down).
    pub fn shared(store: u32, w: u64, w_new: u64) -> u32 {
        if w_new == 0 {
            return 0;
        }
        (store as u128 * w as u128 / w_new as u128).min(u32::MAX as u128) as u32
    }

    /// The share of its cell's capacity a band at `effort` lives on this year: 1 + its
    /// harvest's spread times the cell's `shock`, at least 0 (#13).
    pub fn harvest(&self, effort: f64, cell_cv: f64, shock: f64) -> f64 {
        harvest_share(self.forage_risk_ratio, effort, cell_cv, shock)
    }
}

impl Mechanism for Growth {
    fn spec(&self) -> MechanismSpec {
        MechanismSpec {
            name: "growth",
            period_ticks: self.period_ticks,
            reads: vec![
                Component::Population,
                Component::Capacity,
                Component::AgentTraits,
                Component::Forcing,
            ],
            writes: vec![Component::AgentWeight, Component::AgentTraits],
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
        // Tiles are runs of whole cells: draw each cell's shock, and its peopled neighbors'
        // (spread, shock), once.
        let mut shock: Option<(usize, f64)> = None;
        let (mut near, mut shares): (Vec<(f64, f64)>, Vec<f64>) = (Vec::new(), Vec::new());
        let risky = !sense.harvest_cv.is_empty();
        for i in tile {
            let cell = agents.cell[i] as usize;
            let w = agents.weight[i];
            let pop = sense.population[cell] as f64;
            let effort = Traits::share(agents.cultivation[i]);
            let cv = if risky { sense.harvest_cv[cell] } else { 0.0 };
            let k = if cv > 0.0 || (risky && self.sharing > 0.0) {
                let z = match shock {
                    Some((c, z)) if c == cell => z,
                    _ => {
                        let z = harvest_at(
                            sense.seed,
                            cell as u32,
                            sense.tick,
                            self.harvest_correlation,
                            self.harvest_persistence,
                        );
                        shock = Some((cell, z));
                        near.clear();
                        if self.sharing > 0.0 {
                            near.extend(
                                habitable_neighbors(sense, cell as u32)
                                    .filter(|&c| sense.cell_population[c as usize] > 0)
                                    .map(|c| {
                                        let zc = harvest_at(
                                            sense.seed,
                                            c,
                                            sense.tick,
                                            self.harvest_correlation,
                                            self.harvest_persistence,
                                        );
                                        (sense.harvest_cv[c as usize], zc)
                                    }),
                            );
                        }
                        z
                    }
                };
                let own = self.harvest(effort, cv, z);
                let share = if near.is_empty() {
                    own
                } else {
                    shares.clear();
                    shares.extend(near.iter().map(|&(cvc, zc)| self.harvest(effort, cvc, zc)));
                    pooled_harvest(own, &shares, self.sharing_of(agents.store[i], agents.herding[i]))
                };
                if share == 1.0 { sense.capacity[cell] as f64 } else { sense.capacity[cell] as f64 * share }
            } else {
                sense.capacity[cell] as f64
            };
            // Foragers with nothing stored have nothing to put by or eat from.
            let k = if (agents.store[i] != 0 || (self.storage_years > 0.0 && effort > 0.0)) && pop > 0.0 {
                let fed = k / pop;
                let (eaten, kept) = self.stored(effort, Traits::share(agents.store[i]), fed);
                agents.store[i] = Traits::from_years(kept);
                // The capacity changes only where the store feeds people.
                if eaten > fed { eaten * pop } else { k }
            } else {
                k
            };
            let draw = rng.block(agents.id[i].0, t, 0);
            let (born, died) = if net_only {
                let net = if k > 0.0 { self.rate(effort) * (1.0 - pop / k) } else { -1.0 };
                let unfed = if pop > k { 1.0 - k.max(0.0) / pop } else { 0.0 };
                let dw = oikos_math::finite(net.max(-unfed) * w as f64);
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
            if agents.store[i] != 0 {
                agents.store[i] = Self::shared(agents.store[i], w, w + born - died);
            }
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
/// wealth, by leaders whose decisions others accept (Johnson 1982). A farmer's daughter
/// that stays in her cell stays in her settlement while it holds fewer people than that
/// (mobile foragers' daughters camp alone, as foragers never join a settlement), and a
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
    /// (wild_yield_ratio, wild_herd_ratio), for [`Sense::capacity_joined`].
    pub yields: (f64, f64),
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
            yields: (p.wild_yield_ratio, p.wild_herd_ratio),
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

    /// Where a daughter group of `weight` people and practice `t` goes from its full
    /// `cell`: a random habitable neighbor if it explores, else the least crowded of the
    /// cell and its habitable neighbors, each neighbor judged by what its land would feed
    /// with her there ([`Sense::capacity_joined`]). Where catchments reach beyond cells,
    /// the cells she may go to lie beyond hers ([`Catchments::away`], #50).
    fn destination(
        &self,
        sense: &Sense<'_>,
        rng: &Philox4x32,
        explore: Probability,
        id: EntityId,
        cell: u32,
        (weight, t): (u64, &Traits),
    ) -> u32 {
        let ns = habitable_away(sense, cell);
        let open = ns.clone().count();
        if open == 0 {
            return cell;
        }
        let draws = rng.block(id.0, sense.tick.0, 0);
        if explore.hit(draws[0]) {
            return ns.clone().nth((((draws[1] as u64) * (open as u64)) >> 32) as usize).unwrap();
        }
        let cells = std::iter::once(cell).chain(ns);
        if !sense.farming {
            return least_crowded(sense, cells).unwrap();
        }
        let capacity = |c: u32| {
            if c == cell {
                sense.capacity[c as usize]
            } else {
                sense.capacity_joined(c, weight, t, self.yields)
            }
        };
        least_crowded_by(sense, cells, capacity).unwrap().0
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
                let traits = part.agents.traits(g);
                let dest = if room {
                    cell
                } else {
                    self.destination(sense, &migration_rng, explore, id, cell, (w / 2, &traits))
                };
                // A daughter who stays in her cell stays in her settlement while it has room;
                // mobile foragers found none.
                let stays = self.village != 0
                    && part.agents.cultivation[g] != 0
                    && dest == cell
                    && !self
                        .crowded(&part.with_stayed(cell, part.agents.settlement_of(cell_range.clone(), g)));
                let take = w / 2;
                let half = Agents::split_weight(&mut w, take);
                part.agents.weight[g] = w;
                part.stats.fissions += 1;
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
/// share has somewhere to go moves to the least crowded habitable neighbor, if that
/// neighbor is less crowded and can feed the group (its population, last step, plus the
/// group within its capacity). Each neighbor is judged by what its land would feed with
/// the group there ([`Sense::capacity_joined`], #45): herders look for grazing, which
/// pasture without forage offers them and not foragers, and nobody goes where neither
/// forage nor pasture can feed them. A res-5 cell is about one band's territory. Repeated, this is
/// the wave of advance by which foragers fill a landscape (Fisher-KPP; Ammerman and
/// Cavalli-Sforza 1984), and it works where a cell holds a single band, so fission by
/// group size never fires (res-5 cells feed ~20-50 foragers). Where catchments are not
/// cells (#50), crowding is the catchment's, the group goes to a neighboring catchment
/// ([`Catchments::away`]), and each cell buds with `probability` times its share of the
/// catchment's people, so a catchment buds as often as a cell did.
///
/// Herders travel farther (#51, ADR-0115): a band whose group cannot go to the least
/// crowded neighbor goes instead, with the chance that it lives by domestic herds (its
/// herding effort times its herd's domestic share), to the nearest place within `reach_km`
/// over habitable land, from catchment to neighboring catchment, that is less crowded and
/// can feed the group and its flocks, the least crowded of the nearest such. People move as short a way
/// as will do (intervening opportunities, Stouffer 1940), and herders cross land that
/// feeds them nothing to reach grazing beyond, as Egypt's desert groups moved 150-200 km
/// between the oases and the Nile (Linseele et al. 2014). `reach_km` 0 switches it off.
///
/// People judge the land by the year they are living (#13): crowding, a neighbor's room
/// and how crowded it is are reckoned on what the year's harvest feeds
/// ([`harvest_share`]), as far as `bad_years` weighs ([`Dispersal::this_year`]), so a bad
/// year crowds a cell and sends a group toward a neighbor the rain favoured. Moving is the
/// first answer to a bad year (Halstead and O'Shea 1989): foragers leave camps whose
/// returns have fallen and herders take their flocks to where the rain fell (Kelly 2013),
/// and people weigh what they have just lived through heavily (Hertwig et al. 2004). The
/// origin is judged at its people's mean cultivation effort, a neighbor at the group's.
/// A group with food stored stays through a bad year ([`Dispersal::stays`]): it eats from
/// its store, and its fields and stores tie it down. `bad_years` 0 judges by the mean,
/// as the M1 records have it (ADR-0125): judging by the year empties Egypt's deserts of
/// herds, and with years independent a move takes a group to land the next year treats
/// as any other. Places that feed nobody are no destination.
#[derive(Clone, Debug)]
pub struct Dispersal {
    pub period_ticks: u64,
    pub crowding: f64,
    pub probability: f64,
    pub fraction: f64,
    pub reach_km: f64,
    /// (wild_yield_ratio, wild_herd_ratio), for [`Sense::capacity_joined`].
    pub yields: (f64, f64),
    pub bad_years: f64,
    pub forage_risk_ratio: f64,
    pub harvest_correlation: f64,
    pub harvest_persistence: f64,
    pub after: Vec<&'static str>,
}

impl Dispersal {
    /// What a capacity of `k` feeds as people judge it in a year whose harvest feeds `share`
    /// of it: `k` moved toward `k * share` as far as `bad_years` weighs.
    pub fn this_year(&self, k: u64, share: f64) -> u64 {
        if share == 1.0 || self.bad_years == 0.0 {
            return k;
        }
        (k as f64 * (1.0 + self.bad_years * (share - 1.0))).max(0.0) as u64
    }

    /// Whether a group holding `store` stays in a year whose harvest feeds `share` of its
    /// cell's capacity: it does in a bad year while it has food stored.
    pub fn stays(store: u32, share: f64) -> bool {
        store > 0 && share < 1.0
    }

    /// The share of its capacity `cell`'s harvest feeds this year for a band at `effort`
    /// (1 without harvest risk, or where the year does not weigh).
    fn share(&self, sense: &Sense<'_>, cell: u32, effort: f64) -> f64 {
        if self.bad_years == 0.0 || sense.harvest_cv.is_empty() {
            return 1.0;
        }
        let cv = sense.harvest_cv[cell as usize];
        if cv == 0.0 {
            return 1.0;
        }
        let z = harvest_at(sense.seed, cell, sense.tick, self.harvest_correlation, self.harvest_persistence);
        harvest_share(self.forage_risk_ratio, effort, cv, z)
    }

    /// `cell`'s capacity as its people judge it this year, at their mean cultivation effort.
    fn judged(&self, sense: &Sense<'_>, cell: u32) -> u64 {
        let c = cell as usize;
        let k = sense.capacity[c];
        if self.bad_years == 0.0 || sense.harvest_cv.is_empty() {
            return k;
        }
        let effort = match sense.population[c] {
            0 => 0.0,
            n => sense.effort_sum[c] as f64 / (n as f64 * SHARE_ONE as f64),
        };
        self.this_year(k, self.share(sense, cell, effort))
    }
    /// The chance per period that a crowded cell of `own` people in a catchment of
    /// `catchment` buds off a group (#50): `probability` times its share of the catchment's
    /// people, so a catchment buds as often as a cell did.
    pub fn chance(&self, own: u64, catchment: u64) -> f64 {
        if own == 0 {
            return 0.0;
        }
        if own >= catchment {
            return self.probability;
        }
        self.probability * (own as f64 / catchment as f64)
    }

    /// A habitable cell at least `crowding` full (people last step, capacity `k`).
    fn crowded(&self, sense: &Sense<'_>, cell: u32, k: u64) -> bool {
        let n = sense.population[cell as usize];
        sense.habitable[cell as usize] && n as f64 >= self.crowding * k as f64
    }

    /// The least crowded habitable neighbor of `cell` (beyond its catchment where they reach
    /// beyond cells, [`Catchments::away`]) by the capacities `capacity(c)`, if it is less
    /// crowded than `cell` at capacity `k`, and its capacity.
    fn target(
        &self,
        sense: &Sense<'_>,
        (cell, k): (u32, u64),
        capacity: impl Fn(u32) -> u64,
    ) -> Option<(u32, u64)> {
        let n = sense.population[cell as usize];
        let cells = habitable_away(sense, cell).map(|c| (c, sense.population[c as usize], capacity(c)));
        let (dest, kd) = least_crowded_among(cells, true)?;
        let nd = sense.population[dest as usize];
        (nd as u128 * (k as u128) < n as u128 * kd as u128).then_some((dest, kd))
    }

    /// How many steps from catchment to catchment `reach_km` spans (0: off, or where the
    /// grid's spacing is unknown).
    pub fn reach(&self, step_km: f64) -> usize {
        if self.reach_km <= 0.0 || step_km <= 0.0 {
            return 0;
        }
        (self.reach_km / step_km + 0.5) as usize
    }

    /// Where a group of `take` people leaving `cell` (capacity `k`) goes within `reach` steps
    /// over habitable land, from catchment to neighboring catchment: the least crowded, by
    /// the capacities `capacity(c)`, of the nearest ring's places less crowded than `cell`
    /// with room for it. `search` holds the catchments seen and the rings, reused between
    /// groups.
    fn farther(
        &self,
        sense: &Sense<'_>,
        (cell, k, reach, take): (u32, u64, usize, u64),
        capacity: impl Fn(u32) -> u64,
        search: &mut Search,
    ) -> Option<u32> {
        let (n, k) = (sense.population[cell as usize] as u128, k as u128);
        let trivial = sense.catchments.is_trivial();
        let key = |c: u32| if trivial { c } else { sense.catchments.of(c as usize) as u32 };
        let Search { seen, ring, next } = search;
        seen.clear();
        seen.push(key(cell));
        ring.clear();
        ring.push(cell);
        for _ in 0..reach {
            // Each catchment of the next ring once, at its first cell reached (in order).
            next.clear();
            next.extend(ring.iter().flat_map(|&c| habitable_away(sense, c)));
            next.sort_unstable();
            next.dedup();
            let before = seen.len();
            next.retain(|&c| {
                let b = key(c);
                if seen[..before].binary_search(&b).is_ok() || seen[before..].contains(&b) {
                    return false;
                }
                seen.push(b);
                true
            });
            if next.is_empty() {
                return None;
            }
            seen.sort_unstable();
            std::mem::swap(ring, next);
            let mut best: Option<(u32, u128, u128)> = None;
            for &c in ring.iter() {
                let (nc, kc) = (sense.population[c as usize], capacity(c));
                let room = take <= kc.saturating_sub(nc);
                let (nc, kc) = (nc as u128, kc as u128);
                if room && nc * k < n * kc && best.is_none_or(|(_, bn, bk)| nc * bk < bn * kc) {
                    best = Some((c, nc, kc));
                }
            }
            if let Some((c, _, _)) = best {
                return Some(c);
            }
        }
        None
    }

    pub fn from_params(p: &Params) -> Dispersal {
        Dispersal {
            period_ticks: p.period_ticks,
            crowding: p.dispersal_crowding,
            probability: p.dispersal_probability,
            fraction: p.dispersal_fraction,
            reach_km: p.herder_reach_km,
            yields: (p.wild_yield_ratio, p.wild_herd_ratio),
            bad_years: p.bad_year_moves,
            forage_risk_ratio: p.forage_risk_ratio,
            harvest_correlation: p.harvest_correlation,
            harvest_persistence: p.harvest_persistence,
            after: vec!["fission"],
        }
    }
}

/// Buffers of [`Dispersal`]'s search beyond the neighbors, reused within a tile.
#[derive(Default)]
struct Search {
    seen: Vec<u32>,
    ring: Vec<u32>,
    next: Vec<u32>,
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
        let reach = if sense.farming { self.reach(sense.catchments.step_km()) } else { 0 };
        let mut search = Search::default();
        // Tiles hold whole cells; agents spawned this step lie past the tile.
        let mut i = tile.start;
        while i < tile.end {
            let cell = part.agents.cell[i];
            let end = i + part.agents.cell[i..tile.end].iter().take_while(|&&c| c == cell).count();
            let go = if sense.catchments.is_trivial() {
                go
            } else {
                let c = cell as usize;
                Probability::from_f64(self.chance(sense.cell_population[c], sense.population[c]))
            };
            let k = self.judged(sense, cell);
            if self.crowded(sense, cell, k) && go.hit(rng.u32(cell as u64, t, 0, 0)) {
                // Without farming a neighbor's capacity is its residents' whoever joins them,
                // so the target is the same for every band.
                let residents =
                    (!sense.farming).then(|| self.target(sense, (cell, k), |c| sense.capacity[c as usize]));
                // The first band (canonical order) whose group has somewhere to go buds off.
                for a in (residents != Some(None)).then_some(i..end).into_iter().flatten() {
                    let w = part.agents.weight[a];
                    let take = (w as f64 * self.fraction) as u64;
                    if take == 0 {
                        continue;
                    }
                    let effort = Traits::share(part.agents.cultivation[a]);
                    let store = part.agents.store[a];
                    if store > 0
                        && self.bad_years > 0.0
                        && Self::stays(store, self.share(sense, cell, effort))
                    {
                        continue;
                    }
                    let year = |c: u32, kc: u64| self.this_year(kc, self.share(sense, c, effort));
                    let target = residents.unwrap_or_else(|| {
                        let traits = part.agents.traits(a);
                        self.target(sense, (cell, k), |c| {
                            year(c, sense.capacity_joined(c, take, &traits, self.yields))
                        })
                    });
                    let near = target
                        .filter(|&(dest, kd)| take <= kd.saturating_sub(sense.population[dest as usize]))
                        .map(|(dest, _)| dest);
                    let dest = near.or_else(|| {
                        let herds = Traits::share(part.agents.herding[a])
                            * Traits::share(part.agents.herd_domestication[a]);
                        if reach == 0 || herds <= 0.0 {
                            return None;
                        }
                        let id = part.agents.id[a].0;
                        if !Probability::from_f64(herds).hit(rng.u32(id, t, 1, 0)) {
                            return None;
                        }
                        let traits = part.agents.traits(a);
                        let capacity = |c: u32| year(c, sense.capacity_joined(c, take, &traits, self.yields));
                        self.farther(sense, (cell, k, reach, take), capacity, &mut search)
                    });
                    if let Some(dest) = dest {
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
///
/// Where catchments reach beyond cells (#50), the largest settlement of another cell in the
/// group's catchment that is larger than its own and than any it could join in its cell,
/// and has room for it, draws the group there, to live in it as a group of its own (beside
/// it, where it is a lone group, to join it as the groups of a cell do):
/// farmers walked to fields an hour from home, so a village drew its people from the land
/// around it, more than a cell smaller than that land could feed. Each cell's largest
/// settlement is the last commit's, so several groups may join one in a step, which then
/// sheds groups as any settlement past its limit does.
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

    /// The largest settlement of another cell in `cell`'s catchment with more people than
    /// `than` that has room for a group of `w` people with practice `t` (the first on ties),
    /// and its cell.
    fn beyond(
        &self,
        sense: &Sense<'_>,
        cell: u32,
        than: u64,
        w: u64,
        t: Traits,
    ) -> Option<(u32, Settlement)> {
        let mut best: Option<(u32, Settlement)> = None;
        for c in sense.catchments.mates(cell as usize) {
            let s = sense.largest[c];
            if s.people > than
                && best.is_none_or(|(_, b)| s.people > b.people)
                && sense.habitable[c]
                && self.settles(&s, w, t)
            {
                best = Some((c as u32, s));
            }
        }
        best
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
            reads: vec![Component::AgentTraits, Component::Forcing],
            writes: vec![
                Component::AgentWeight,
                Component::AgentTraits,
                Component::AgentSettlement,
                Component::AgentSpawn,
            ],
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
            let here = target.map_or(own.people, |(s, _)| s.people.max(own.people));
            if let Some((dest, s)) = self.beyond(sense, cell, here, w, ta) {
                part.agents.weight[a] = 0;
                part.spawn_settled(cell, dest, w, ta, s.label);
                continue;
            }
            let Some((s, group)) = target else { continue };
            let Some(b) = group else {
                part.agents.settlement[a] = part.label_of(s.first);
                continue;
            };
            let agents = &mut *part.agents;
            let wb = agents.weight[b];
            let (tb, ta) = (agents.traits(b), agents.traits(a));
            agents.set_traits(b, Traits::pooled((wb, tb), (w, ta)));
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
///   Bands sow emmer and einkorn with the crop (#38), in the same fields under the same
///   effort, and each wheat is selected at its own fraction of the crop's selection, as
///   the founder crops were domesticated at different paces (Allaby et al. 2017; Fuller
///   et al. 2014). Gene flow pulls a wheat back only where its wild ancestor grows (the
///   `wild_wheats` pack; everywhere without it): wild emmer and einkorn range far less
///   widely than wild barley, and villages beyond their ranges sowed seed brought from
///   them, with no wild stands nearby to mix back in (Willcox 2005). Selection is what the harvest does: reaping ripe ears with sickles
///   selects strongly, beating or uprooting them weakly (Hillman and Davies 1990), so the
///   rates stand for how people harvested each crop, a practice learned with cultivation
///   itself. The crop's own share is barley's, which stands for the package in carrying
///   capacity; the wheats' shares ride along, for the records of their rachis.
/// - **Seed exchange** (#48, ADR-0102). Seed moves between people, not only with them:
///   farmers renew, replace and gift seed through kinship, marriage and exchange
///   partners, out of reciprocity and the trust those ties carry (seed networks:
///   Pautasso et al. 2013); payoff bias in whose seed is taken is not modelled. So a band
///   replaces `seed_exchange` times its effort of its seed stock, each period, with seed
///   at the domestic shares sown by the people it learns from: its cell's and, by
///   `neighbor_learning`, the cells around it, by the effort each sows. Villages where
///   few farm then sow domestic crops bred elsewhere, as the oases of the Syrian steppe
///   did; and the core's crops take in some of the wilder seed around them. Off by
///   default, a hypothesis for calibration to weigh.
///
/// - **Herding** (#37, ADR-0087). A band also moves its share of effort managing sheep and
///   goats toward a target: `herding_weight` times the land it can herd on, the wild
///   herds' range and, with domestic animals, the pasture beyond it in proportion to their
///   share; and, wherever it can herd at all, need, as for cultivation: by
///   `herd_pressure_weight`, the population pressure of people beyond what foraging feeds,
///   in proportion to the herd's domestic share (#44), as wild animals cannot be worked
///   harder to feed more people and specialized herding came with docile domestic stock
///   (Zeder 2008). Where rain grows no crops, crowded people turn to their flocks, which graze
///   wherever there is grass, along a desert's wadis too
///   ([`crate::environment::pasture_share`]): so herders, not farmers, cross deserts, as
///   goat herding reached Egypt through the Negev and Sinai before farming did. It learns it as it learns cultivation, from the share of herding among the
///   people around it; both efforts move from one snapshot within one budget
///   ([`Cultivation::share_effort`]). Management breeds herds domestic, from wild animals
///   only where they range, and gene flow pulls them back only there
///   ([`Cultivation::domesticate_herd`]).
/// - **Stock exchange** (#51). Animals pass between people as seed does: herders give,
///   lend and marry with them along kinship, marriage and friendship ties, which pool the
///   risk of losing a herd to drought or disease (Aktipis, Cronk and de Aguiar 2011). So
///   a band replaces `herd_exchange` of its stock each period with animals at the domestic
///   share of the herds of the people it is tied to: its cell's and, by
///   `neighbor_learning`, the cells around it, by the effort each herds
///   ([`Cultivation::exchange_stock`]). A band that herds nothing takes animals too, and
///   with domestic ones it can graze pasture where no wild herds lived, so hunters beside
///   herders may start to keep a few, as Egypt's desert hunters did before anyone farmed
///   there (Linseele et al. 2014). Off by default, a hypothesis for calibration to weigh.
/// - **A reserve of animals** (#51, ADR-0114). Animals are food that keeps, so a band with
///   domestic stock keeps at least `herd_reserve` times its domestic share of effort on
///   it wherever its land grazes any, against lean times, as hunters kept a handful of
///   goats among their game (Linseele et al. 2014; Dahl and Hjort 1976). It is a floor
///   tended at once ([`Cultivation::kept`]), not a target approached by copying others:
///   animals a band holds are its own. Off by default.
///
/// Cultivation and herding feed back through carrying capacity
/// ([`crate::environment::capacity_of`]): domestic crops and herds feed more people per
/// area, so populations grow, pressure rises and effort with it (Rindos 1984). Toy worlds
/// (no climate pack) never cultivate or herd.
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
    pub seed_exchange: f64,
    /// Selection on emmer's and einkorn's domestic traits under full effort (#38).
    pub wheat_selection: [f64; 2],
    pub herding_weight: f64,
    pub herd_pressure_weight: f64,
    pub herd_selection: f64,
    pub herd_wild_frequency: f64,
    pub herd_introgression: f64,
    pub herd_exchange: f64,
    pub herd_reserve: f64,
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
            seed_exchange: p.seed_exchange,
            wheat_selection: [
                p.emmer_selection_ratio * p.selection_coefficient,
                p.einkorn_selection_ratio * p.selection_coefficient,
            ],
            herding_weight: p.herding_weight,
            herd_pressure_weight: p.herd_pressure_weight,
            herd_selection: p.herd_selection,
            herd_wild_frequency: p.herd_wild_frequency,
            herd_introgression: p.herd_introgression,
            herd_exchange: p.herd_exchange,
            herd_reserve: p.herd_reserve,
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
        let pressure = self.pressure(people, forage);
        let drive = self.pressure_weight * pressure
            + self.stress_weight * stress
            + self.abundance_weight * suitability;
        let reliability = (1.0 - self.variability_weight * variability).clamp(0.0, 1.0);
        oikos_math::finite(drive * reliability).clamp(0.0, 1.0)
    }

    /// Population pressure in a cell, in [0, 1]: its `people` beyond `crowding` times what
    /// foraging feeds there (`forage`), full at one capacity more.
    pub fn pressure(&self, people: u64, forage: u64) -> f64 {
        let crowding = people as f64 / forage.max(1) as f64;
        oikos_math::finite(crowding - self.crowding).clamp(0.0, 1.0)
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
    /// full-farming capacities `(farm, flood)` of rain-fed land and floodplain, its forager
    /// capacity and its people's sums of effort and of effort times domestic share.
    pub fn payoff(&self, (farm, flood): (f64, f64), forage: u64, effort_sum: u64, crop_sum: u64) -> f64 {
        let domestic = if effort_sum == 0 { 0.0 } else { crop_sum as f64 / effort_sum as f64 };
        let w = self.wild_yield_ratio;
        oikos_math::finite((farm * (w + (1.0 - w) * domestic) + flood * domestic) / forage.max(1) as f64)
    }

    /// The suitability a band with domestic share `domestic` sees in its land: wild
    /// cereals' `suitability`, and the `floodplain` it can sow only with the domestic crops
    /// it has (ADR-0085), at most 1.
    pub fn sowable(suitability: f64, floodplain: f64, domestic: f64) -> f64 {
        (suitability + floodplain * domestic).min(1.0)
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
        land: (f64, f64),
        forage: u64,
        people: u64,
        effort_sum: u64,
        crop_sum: u64,
    ) -> (f64, f64) {
        let effort = if people == 0 { 0.0 } else { effort_sum as f64 / (people as f64 * SHARE_ONE as f64) };
        let payoff = if effort_sum > 0 && self.success_bias != 0.0 {
            self.payoff(land, forage, effort_sum, crop_sum)
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

    /// [`Cultivation::example`] for a band in `cell`: its cell and its neighbors (its
    /// catchment and the neighboring ones where cells are not their own, #50).
    fn example_at(&self, sense: &Sense<'_>, cell: u32) -> Option<f64> {
        let cached = sense.models.len() == sense.population.len()
            && sense.models_for == (self.success_bias, self.wild_yield_ratio);
        let at = |c: u32, contact: f64| {
            let c = c as usize;
            let models = if cached {
                sense.models[c]
            } else {
                let land = (sense.farm[c], sense.flood[c]);
                self.cell_models(
                    land,
                    sense.forage_capacity[c],
                    sense.population[c],
                    sense.effort_sum[c],
                    sense.crop_sum[c],
                )
            };
            (contact, models)
        };
        let neighbors =
            sense.catchments.away(sense.grid, cell as usize).iter().map(|&c| at(c, self.neighbor_learning));
        Self::example_of(std::iter::once(at(cell, 1.0)).chain(neighbors))
    }

    /// The share of effort a band with a herd `domestic` aims to spend herding (#37):
    /// `herding_weight` times the land it can herd on, the wild herds' `range` and, with
    /// domestic animals, the `pasture` beyond it in proportion to their share; and, on any
    /// such land, `herd_pressure_weight` times the cell's population `pressure` times that
    /// share (#44): wild herds cannot be worked harder to feed more people. On any such
    /// land a band keeps at least `herd_reserve` times that share (#51): domestic animals
    /// are food that keeps, a reserve against lean times however little the land repays
    /// them. At most 1.
    pub fn herding_target(&self, range: f64, pasture: f64, domestic: f64, pressure: f64) -> f64 {
        let land = (range + (pasture - range).max(0.0) * domestic).min(1.0);
        if land <= 0.0 {
            return 0.0;
        }
        let need = self.herd_pressure_weight * pressure * domestic;
        let reserve = self.herd_reserve * domestic;
        oikos_math::finite((self.herding_weight * land + need).max(reserve)).clamp(0.0, 1.0)
    }

    /// The least effort a band keeps on a herd `domestic` (#51): `herd_reserve` times that
    /// share wherever its land grazes any. Animals it holds are tended at once, not learned
    /// from others first, as the reserve is food it owns.
    pub fn kept(&self, range: f64, pasture: f64, domestic: f64) -> f64 {
        let land = (range + (pasture - range).max(0.0) * domestic).min(1.0);
        if land <= 0.0 || self.herd_reserve == 0.0 {
            return 0.0;
        }
        oikos_math::finite(self.herd_reserve * domestic).clamp(0.0, 1.0)
    }

    /// Cultivation and herding effort within one effort budget: proportionally less of
    /// each where together they would exceed it (#37).
    pub fn share_effort(cultivation: f64, herding: f64) -> (f64, f64) {
        let total = cultivation + herding;
        if total <= 1.0 { (cultivation, herding) } else { (cultivation / total, herding / total) }
    }

    /// Domestic share of a herd after one period at herding `effort`, from `d` (#37):
    /// management selects for domestic traits as sowing does, from the wild animals'
    /// domestic-type share only where wild herds `range`, and gene flow from them pulls the
    /// share back there; herds kept beyond the range keep their traits.
    pub fn domesticate_herd(&self, d: f64, effort: f64, range: f64) -> f64 {
        let gain = self.herd_selection * effort * d.max(self.herd_wild_frequency * range) * (1.0 - d);
        let loss = self.herd_introgression * (1.0 - effort) * d * range;
        oikos_math::finite(d + gain - loss).clamp(0.0, 1.0)
    }

    /// The domestic share of the herds of the people a band in `cell` is tied to (#51): its
    /// cell's and, by `neighbor_learning`, the cells around it (catchments, #50), by the
    /// effort each herds. None if nobody herds there.
    fn herds_seen(&self, sense: &Sense<'_>, cell: u32) -> Option<f64> {
        let at = |c: u32| (sense.herd_sum[c as usize] as f64, sense.herd_crop_sum[c as usize] as f64);
        let (mut herded, mut domestic) = at(cell);
        if self.neighbor_learning != 0.0 {
            for &c in sense.catchments.away(sense.grid, cell as usize) {
                let (h, d) = at(c);
                herded += self.neighbor_learning * h;
                domestic += self.neighbor_learning * d;
            }
        }
        (herded > 0.0).then(|| oikos_math::finite(domestic / herded))
    }

    /// A herd's domestic share after stock exchange (#51): `herd_exchange` of a band's
    /// stock is replaced by animals at the share `seen` among the herds of those it is tied
    /// to. A band that herds nothing takes animals too, to start a herd with.
    pub fn exchange_stock(&self, d: f64, seen: f64) -> f64 {
        oikos_math::finite(d + self.herd_exchange * (seen - d)).clamp(0.0, 1.0)
    }

    /// The share of herding effort among the people a band in `cell` learns from: its
    /// cell's and, by `neighbor_learning`, the cells around it (#37; catchments where cells
    /// are not their own, #50). None if it sees nobody.
    fn herding_seen(&self, sense: &Sense<'_>, cell: u32) -> Option<f64> {
        let at = |c: u32| {
            (sense.herd_sum[c as usize] as f64, sense.population[c as usize] as f64 * SHARE_ONE as f64)
        };
        let (mut herd, mut people) = at(cell);
        if self.neighbor_learning != 0.0 {
            for &c in sense.catchments.away(sense.grid, cell as usize) {
                let (h, n) = at(c);
                herd += self.neighbor_learning * h;
                people += self.neighbor_learning * n;
            }
        }
        (people > 0.0).then(|| oikos_math::finite(herd / people))
    }

    /// The domestic shares of the crop, emmer and einkorn sown by the people a band in
    /// `cell` exchanges seed with (#48): its cell's and, by `neighbor_learning`, the cells
    /// around it (catchments, #50), kin and marriage partners as for learning, by the effort
    /// each sows. None if nobody sows there.
    fn seed_seen(&self, sense: &Sense<'_>, cell: u32) -> Option<[f64; 3]> {
        let at = |c: u32| {
            let (c, [em, ek]) = (c as usize, sense.wheat_sum[c as usize]);
            [sense.effort_sum[c], sense.crop_sum[c], em, ek].map(|x| x as f64)
        };
        let mut sums = at(cell);
        if self.neighbor_learning != 0.0 {
            for &c in sense.catchments.away(sense.grid, cell as usize) {
                for (s, x) in sums.iter_mut().zip(at(c)) {
                    *s += self.neighbor_learning * x;
                }
            }
        }
        let [sown, crops @ ..] = sums;
        (sown > 0.0).then(|| crops.map(|x| oikos_math::finite(x / sown)))
    }

    /// A crop's domestic share after seed exchange (#48): a band sowing at `effort` and
    /// share `d` replaces `seed_exchange` times that share of its stock with seed at the
    /// share `seen` among those it exchanges with.
    pub fn exchange_seed(&self, d: f64, effort: f64, seen: f64) -> f64 {
        oikos_math::finite(d + self.seed_exchange * effort * (seen - d)).clamp(0.0, 1.0)
    }

    /// Domestic share of the crop after one period at `effort`, from `d`.
    pub fn domesticate(&self, d: f64, effort: f64) -> f64 {
        self.select(d, effort, self.selection)
    }

    /// Domestic shares of emmer and einkorn after one period at `effort` (#38), in a cell
    /// that is `wild` in each one's wild range: gene flow from wild stands pulls a wheat
    /// back only there, so seed sown beyond the range keeps its traits (Willcox 2005).
    pub fn domesticate_wheats(&self, d: [f64; 2], effort: f64, wild: [f64; 2]) -> [f64; 2] {
        [
            self.select_within(d[0], effort, self.wheat_selection[0], wild[0]),
            self.select_within(d[1], effort, self.wheat_selection[1], wild[1]),
        ]
    }

    fn select(&self, d: f64, effort: f64, selection: f64) -> f64 {
        self.select_within(d, effort, selection, 1.0)
    }

    /// [`Cultivation::select`] where a share `wild` of the land holds the crop's wild
    /// stands. Sown wild seed carries its domestic mutants wherever it is sown.
    fn select_within(&self, d: f64, effort: f64, selection: f64, wild: f64) -> f64 {
        let gain = selection * effort * d.max(self.wild_frequency) * (1.0 - d);
        let loss = self.introgression * (1.0 - effort) * d * wild;
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
        // Without a wild-wheats pack the wheats grow wild wherever they are sown.
        let (kind, names) = crate::environment::WILD_WHEATS;
        let wild_wheats = names.map(|name| sense.inputs.field(kind, name).ok());
        // Learning from the own cell by how many plant (ADR-0065) is computed as it was.
        let own_cell_only = self.own_cell_only();
        let mut example: Option<(usize, Option<f64>)> = None; // per cell, only where needed
        let mut herders: Option<(usize, Option<f64>)> = None; // likewise for herding
        let mut seeds: Option<(usize, Option<[f64; 3]>)> = None; // likewise for seed exchange
        let mut stock: Option<(usize, Option<f64>)> = None; // likewise for stock exchange
        for i in tile {
            let cell = agents.cell[i] as usize;
            let traits = agents.traits(i);
            let target = self.target_effort(
                sense.population[cell],
                sense.forage_capacity[cell],
                Self::sowable(
                    sense.suitability[cell],
                    sense.floodplain[cell],
                    Traits::share(traits.domestication),
                ),
                sense.climate_stress[cell],
                sense.climate_variability[cell],
            );
            let herd_target = self.herding_target(
                sense.range[cell],
                sense.pasture[cell],
                Traits::share(traits.herd_domestication),
                self.pressure(sense.population[cell], sense.forage_capacity[cell]),
            );
            // Animals from herders nearby (#51), which may start a herd where none is kept.
            let traded = if self.herd_exchange == 0.0 {
                None
            } else {
                let seen = match stock {
                    Some((at, seen)) if at == cell => seen,
                    _ => self.herds_seen(sense, cell as u32),
                };
                stock = Some((cell, seen));
                seen
            };
            if target == 0.0
                && herd_target == 0.0
                && traits.cultivation == 0
                && traits.domestication == 0
                && traits.herding == 0
                && traits.herd_domestication == 0
                && traits.wheat_domestication == [0; 2]
                && traded.is_none_or(|seen| seen == 0.0)
            {
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
            let (h, dh) = (Traits::share(traits.herding), Traits::share(traits.herd_domestication));
            let next_h = if herd_target == 0.0 && traits.herding == 0 {
                0.0
            } else {
                let seen = match herders {
                    Some((at, seen)) if at == cell => seen,
                    _ => self.herding_seen(sense, cell as u32),
                };
                herders = Some((cell, seen));
                self.adopt(h, herd_target, seen.unwrap_or(h)).max(self.kept(
                    sense.range[cell],
                    sense.pasture[cell],
                    Traits::share(traits.herd_domestication),
                ))
            };
            let (next_e, next_h) = Self::share_effort(self.adopt(e, target, seen), next_h);
            let cultivation = Traits::toward(traits.cultivation, next_e);
            // Rounding toward each change may overshoot the budget by a unit or two.
            let herding = Traits::toward(traits.herding, next_h).min(SHARE_ONE - cultivation);
            let wild = [EMMER, EINKORN].map(|w| {
                wild_wheats[w].as_ref().map_or(1.0, |f| f.get(0, cell).unwrap_or(0.0).clamp(0.0, 1.0))
            });
            let mut wheats = self.domesticate_wheats(traits.wheat_domestication.map(Traits::share), e, wild);
            let mut domestic = self.domesticate(d, e);
            if self.seed_exchange != 0.0 && e > 0.0 {
                let seen = match seeds {
                    Some((at, seen)) if at == cell => seen,
                    _ => self.seed_seen(sense, cell as u32),
                };
                seeds = Some((cell, seen));
                if let Some([crop, emmer, einkorn]) = seen {
                    domestic = self.exchange_seed(domestic, e, crop);
                    wheats = [
                        self.exchange_seed(wheats[EMMER], e, emmer),
                        self.exchange_seed(wheats[EINKORN], e, einkorn),
                    ];
                }
            }
            agents.set_traits(
                i,
                Traits {
                    cultivation,
                    domestication: Traits::toward(traits.domestication, domestic),
                    herding,
                    herd_domestication: Traits::toward(traits.herd_domestication, {
                        let bred = self.domesticate_herd(dh, h, sense.range[cell]);
                        traded.map_or(bred, |seen| self.exchange_stock(bred, seen))
                    }),
                    wheat_domestication: [EMMER, EINKORN]
                        .map(|c| Traits::toward(traits.wheat_domestication[c], wheats[c])),
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
/// Herds are wealth too (#45): animals are owned, inherited and given as bridewealth on
/// open land as well, so pastoralists pass on more (0.43) and are unequal (0.42), and
/// drought, epizootics and raids shake their fortunes. A group's herding effort moves its
/// transmission and shocks toward the pastoral economy's as its owned land does toward
/// the landed one's: the three economies mix by effort (effort left to foraging, effort on
/// owned land, effort herding), per generation.
///
/// Inequality changes nothing else yet: no feedback on fertility, fission, aggregation
/// or conflict. It is updated every period by the equivalent fraction of a generation
/// (tabulated over transmission), so a generation of periods is the generational
/// recursion whatever the period length. Merged groups pool it,
/// people-weighted (the spread between their mean wealth is left out), and split-off
/// groups take their parent's.
#[derive(Clone, Debug)]
pub struct Inequality {
    pub period_ticks: u64,
    /// Transmission per generation of the foraging, landed and pastoral economies, and the
    /// variance of their shocks.
    pub beta: [f64; 3],
    pub shock: [f64; 3],
    /// Per period, at transmission `beta_lo + k / STEPS (beta_hi - beta_lo)`: the share of
    /// the variance kept, and the variance added per unit of a generation's shock.
    pub per_period: Vec<(f64, f64)>,
    pub beta_lo: f64,
    pub beta_hi: f64,
    /// The stored inequality of foragers that the update keeps exactly (most agents, most
    /// of the time), so they are skipped; None if rounding has no such fixed point.
    pub forager_fixed: Option<u32>,
    /// Share of a full store owned like closed-in land, and the store that is full.
    pub stored_wealth: f64,
    pub storage_years: f64,
    pub after: Vec<&'static str>,
}

impl Inequality {
    /// Transmission steps tabulated: linear interpolation between them errs by under 1e-7.
    pub const STEPS: usize = 1024;

    pub fn from_params(p: &Params) -> Inequality {
        let beta = [p.wealth_transmission, p.landed_wealth_transmission, p.pastoral_wealth_transmission];
        assert!(beta.iter().all(|&b| b < 1.0), "validated parameters");
        let generations =
            p.period_ticks as f64 / (oikos_core::time::TICKS_PER_YEAR as f64 * p.generation_years);
        let (beta_lo, beta_hi) =
            (beta.iter().copied().fold(1.0, f64::min), beta.iter().copied().fold(0.0, f64::max));
        let per_period = (0..=Self::STEPS)
            .map(|k| {
                let b = beta_lo + k as f64 / Self::STEPS as f64 * (beta_hi - beta_lo);
                let kept = oikos_math::pow(b * b, generations);
                // A generation of periods adds shock: the geometric sum of what each adds.
                (kept, (1.0 - kept) / (1.0 - b * b))
            })
            .collect();
        let mut m = Inequality {
            period_ticks: p.period_ticks,
            beta,
            shock: [p.wealth_shock, p.landed_wealth_shock, p.pastoral_wealth_shock],
            per_period,
            beta_lo,
            beta_hi,
            forager_fixed: None,
            stored_wealth: p.stored_wealth,
            storage_years: p.storage_years,
            after: vec!["cultivation"],
        };
        let fixed = Self::stored(m.stationary((0.0, 0.0)));
        m.forager_fixed = (m.next_stored(fixed, (0.0, 0.0)) == fixed).then_some(fixed);
        m
    }

    /// Foragers at the inequality they settle at: how new worlds start (ADR-0074).
    pub fn foragers(p: &Params) -> Traits {
        Traits { inequality: Self::stored(Self::from_params(p).stationary((0.0, 0.0))), ..Traits::FORAGER }
    }

    fn stored(v: f64) -> u32 {
        (oikos_math::finite(v) * SHARE_ONE as f64).round().min(u32::MAX as f64) as u32
    }

    /// (share kept, variance added) per period for a group with `land` of its effort on
    /// owned land and `herds` herding (together at most 1).
    fn coefficients(&self, (land, herds): (f64, f64)) -> (f64, f64) {
        let mix = |x: [f64; 3]| (1.0 - land - herds) * x[0] + land * x[1] + herds * x[2];
        let (beta, shock) = (mix(self.beta), mix(self.shock));
        let span = self.beta_hi - self.beta_lo;
        let x = if span > 0.0 {
            ((beta - self.beta_lo) / span).clamp(0.0, 1.0) * Self::STEPS as f64
        } else {
            0.0
        };
        let k = (x as usize).min(Self::STEPS);
        let (kept, scale) = if k == Self::STEPS {
            self.per_period[k]
        } else {
            let ((a0, b0), (a1, b1), f) = (self.per_period[k], self.per_period[k + 1], x - k as f64);
            (a0 + f * (a1 - a0), b0 + f * (b1 - b0))
        };
        (kept, shock * scale)
    }

    /// [`Inequality::next`] of a stored variance, stored.
    pub fn next_stored(&self, v: u32, economy: (f64, f64)) -> u32 {
        Self::stored(self.next(v as f64 / SHARE_ONE as f64, economy))
    }

    /// The variance of log wealth a period after `v`, for a group spending `economy.0` of
    /// its effort on land it owns and `economy.1` herding.
    pub fn next(&self, v: f64, economy: (f64, f64)) -> f64 {
        let (kept, added) = self.coefficients(economy);
        kept * v + added
    }

    /// The variance of log wealth a group of `economy` (see [`Inequality::next`]) settles at.
    pub fn stationary(&self, economy: (f64, f64)) -> f64 {
        let (kept, added) = self.coefficients(economy);
        added / (1.0 - kept)
    }

    /// The share of a group's wealth that is owned and inherited, for a group at
    /// cultivation `effort` on land `closed` in, its store `fill`ed that far: its farmland
    /// where closed in (ADR-0074), and `stored_wealth` of the open rest as far as its
    /// store is full (#13), since a store a household may keep is its own. Both are the
    /// farming economy's: a group that has left farming lives by foragers' levelling
    /// norms, which share out what it still holds (Woodburn 1982), so its store, which
    /// rots away, is not owned.
    pub fn owned(effort: f64, closed: f64, fill: f64, stored_wealth: f64) -> f64 {
        if fill == 0.0 || stored_wealth == 0.0 {
            return effort * closed;
        }
        effort * (closed + stored_wealth * fill * (1.0 - closed))
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
        Self::closure(sense.catchments.away(sense.grid, cell as usize).iter().map(|&c| {
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
        let herding = &agents.herding[tile.clone()];
        let cells = &agents.cell[tile.clone()];
        let stores = &agents.store[tile.clone()];
        let inequality = &mut agents.inequality[tile];
        for (k, v) in inequality.iter_mut().enumerate() {
            let c = cultivation[k];
            if c == 0 && *v == fixed && herding[k] == 0 {
                continue;
            }
            let (h, cell) = (herding[k], cells[k]);
            let owned = if c == 0 {
                0.0
            } else {
                let closed = match closure {
                    Some((at, closed)) if at == cell => closed,
                    _ => Self::closure_of(sense, cell),
                };
                closure = Some((cell, closed));
                let fill =
                    if self.stored_wealth > 0.0 { store_fill(stores[k], self.storage_years) } else { 0.0 };
                Self::owned(Traits::share(c), closed, fill, self.stored_wealth)
            };
            *v = self.next_stored(*v, (owned, Traits::share(h)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::{Land, Practice, capacity_of};
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
    fn run_all(ms: &[&dyn Mechanism], agents: Agents, capacity: u64) -> Agents {
        run_on(ms, agents, &|_| (capacity, None), &Params::default())
    }

    /// [`run_all`] on land where cell `c` gives foragers `ground(c).0` people and, if
    /// `ground(c).1` is some, has a climate, with pasture that feeds that many people under
    /// full herding of domestic herds (no fields, no wild herds); each cell's capacity is
    /// what it feeds its people under their pooled practice, as a world's is.
    fn run_on(
        ms: &[&dyn Mechanism],
        mut agents: Agents,
        ground: &dyn Fn(u32) -> (u64, Option<f64>),
        p: &Params,
    ) -> Agents {
        let grid = Grid::from_base_cell(22, 1).unwrap();
        let n = grid.len();
        let inputs = Inputs::none();
        let farming = ground(0).1.is_some();
        let (mut population, mut sums, mut wheat_sum) = (vec![0; n], vec![[0u64; 4]; n], vec![[0u64; 2]; n]);
        for i in 0..agents.len() {
            let (c, w, t) = (agents.cell[i] as usize, agents.weight[i], agents.traits(i));
            population[c] += w;
            for (s, x) in sums[c].iter_mut().zip(t.practice()) {
                *s += w * x;
            }
            for (s, x) in wheat_sum[c].iter_mut().zip(t.wheat_practice()) {
                *s += w * x;
            }
        }
        let forage: Vec<u64> = (0..n as u32).map(|c| ground(c).0).collect();
        let pasture: Vec<f64> = (0..n as u32).map(|c| ground(c).1.unwrap_or(0.0)).collect();
        let capacity: Vec<u64> = (0..n)
            .map(|c| {
                if !farming || population[c] == 0 {
                    return forage[c];
                }
                let people = (population[c] * SHARE_ONE as u64) as f64;
                let [effort, crop, herding, herd_crop] = sums[c].map(|x| x as f64 / people);
                let land = Land { forage: forage[c], herd_pasture: pasture[c], ..Land::default() };
                capacity_of(&land, &Practice { effort, crop, herding, herd_crop }, p)
            })
            .collect();
        let column = |k: usize| sums.iter().map(|s| s[k]).collect::<Vec<u64>>();
        let (effort_sum, crop_sum, herd_sum, herd_crop_sum) = (column(0), column(1), column(2), column(3));
        let (habitable, zeros) = (vec![true; n], vec![0.0; n]);
        let sense = Sense {
            grid: &grid,
            inputs: &inputs,
            habitable: &habitable,
            restricted: false,
            capacity: &capacity,
            forage_capacity: &forage,
            farming,
            suitability: &zeros,
            climate_stress: &zeros,
            climate_variability: &zeros,
            harvest_cv: &zeros,
            population: &population,
            effort_sum: &effort_sum,
            farm: &zeros,
            floodplain: &zeros,
            flood: &zeros,
            range: &zeros,
            pasture: &zeros,
            range_feeds: &zeros,
            pasture_feeds: &pasture,
            herd_sum: &herd_sum,
            herd_crop_sum: &herd_crop_sum,
            crop_sum: &crop_sum,
            wheat_sum: &wheat_sum,
            models: &[],
            models_for: (0.0, 0.0),
            catchments: &Catchments::default(),
            cell_population: &population,
            largest: &[],
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
    fn a_place_that_feeds_nobody_is_no_destination_and_hides_none() {
        // A failed harvest leaves an empty neighbor feeding nobody (#13): 0 people of 0 is
        // no less crowded than anywhere, so it must not stand first and hide the rest.
        let ranked = |c: &[(u32, u64, u64)]| least_crowded_among(c.iter().copied(), true);
        assert_eq!(ranked(&[(1, 0, 0), (2, 10, 100), (3, 50, 100)]), Some((2, 100)));
        assert_eq!(ranked(&[(1, 0, 0)]), None, "nowhere to go");
        assert_eq!(ranked(&[(1, 5, 0), (2, 10, 100)]), Some((2, 100)));
        assert_eq!(least_crowded_among([(1, 0, 0), (2, 10, 100)], false), Some((1, 0)), "as fission ranks");
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
    fn a_foragers_daughter_camps_alone() {
        // A band of 70 foragers is above its limit (60) and splits; its land has room, so
        // the daughter camps in the cell, but mobile foragers found no settlement (they
        // never join one either).
        let mut scene = Agents::default();
        scene.push(id(1), 0, 70);
        let a = run(&Fission::from_params(&params(3.0)), scene, 1000);
        assert_eq!(a.weight, [35, 35]);
        assert_eq!(a.cell, [0, 0]);
        assert!(a.alone(0..2), "{:?}", a.settlement);
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

    /// Herders with domestic herds, `herding` of their effort, in cell 0.
    fn herders_at_home(people: u64, herding: f64) -> Agents {
        let mut a = Agents::default();
        let t = Traits { herding: Traits::from_share(herding), herd_domestication: ONE, ..Traits::FORAGER };
        a.push_in(id(1), 0, people, t, ALONE);
        a
    }

    fn dispersal() -> (Dispersal, Params) {
        let p = Params {
            dispersal_crowding: 0.5,
            dispersal_probability: 1.0,
            dispersal_fraction: 0.5,
            ..Params::default()
        };
        (Dispersal::from_params(&p), p)
    }

    #[test]
    fn herders_move_onto_pasture_that_feeds_no_foragers() {
        // Cell 0 feeds 50 foragers and its pasture 200 herders; 100 herders spending 0.6
        // of their effort herding fill most of its 140. Around it is pasture that feeds
        // nobody who forages: the 50 who leave can live there by their herds (120).
        let (d, p) = dispersal();
        let ground = |c: u32| if c == 0 { (50, Some(200.0)) } else { (0, Some(200.0)) };
        let a = run_on(&[&d], herders_at_home(100, 0.6), &ground, &p);
        assert_eq!(a.weight, [50, 50], "{:?}", a.cell);
        assert_ne!(a.cell[1], 0, "the group left");
    }

    #[test]
    fn foragers_do_not_move_where_only_herds_could_feed_them() {
        let (d, p) = dispersal();
        let mut home = Agents::default();
        home.push_in(id(1), 0, 150, Traits::FORAGER, ALONE);
        let ground = |c: u32| if c == 0 { (200, Some(200.0)) } else { (0, Some(200.0)) };
        let a = run_on(&[&d], home, &ground, &p);
        assert_eq!(a.weight, [150], "pasture feeds no foragers");
    }

    #[test]
    fn nobody_moves_where_neither_forage_nor_pasture_can_feed_them() {
        let (d, p) = dispersal();
        let ground = |c: u32| if c == 0 { (50, Some(200.0)) } else { (0, Some(0.0)) };
        let a = run_on(&[&d], herders_at_home(100, 0.6), &ground, &p);
        assert_eq!(a.weight, [100], "barren land");
    }

    #[test]
    fn a_herders_daughter_joins_foragers_whose_land_she_can_graze() {
        // 130 herders split (limit 60) where their land feeds 124 (10 forage at 0.4 of
        // their effort, 200 pasture at 0.6). The neighbors' 55 foragers are above the 50
        // they forage, so by their residents' capacity the neighbors are more crowded than
        // home; but their pasture feeds herders, and joined by the daughter's 65 their land
        // feeds about 99, so she goes there.
        let p = Params { exploration_probability: 0.0, ..params(1.0) };
        let mut scene = herders_at_home(130, 0.6);
        for c in 1..7 {
            scene.push_in(EntityId::new(c, 1), c, 55, Traits::FORAGER, ALONE);
        }
        let ground = |c: u32| if c == 0 { (10, Some(200.0)) } else { (50, Some(200.0)) };
        let a = run_on(&[&Fission::from_params(&p)], scene, &ground, &p);
        let daughter = a.len() - 1;
        assert_eq!(a.weight[daughter], 65);
        assert_ne!(a.cell[daughter], 0, "she leaves her full cell");
    }

    #[test]
    fn a_catchment_buds_as_often_as_a_cell_did() {
        let d = Dispersal { probability: 0.3, ..dispersal().0 };
        // Its cells' chances sum to the cell's, and a catchment of one cell has the cell's.
        let total: f64 = [10, 0, 25, 65].iter().map(|&own| d.chance(own, 100)).sum();
        assert!((total - 0.3).abs() < 1e-12);
        assert_eq!(d.chance(40, 40), 0.3);
        assert_eq!(d.chance(0, 0), 0.0);
    }
}
