//! World state and the phased step.
//!
//! Each step is bulk-synchronous:
//! 1. **sense**   — read the previous step's cell populations (a read-only snapshot,
//!    i.e. the halo in a distributed run);
//! 2. **decide**  — each partition updates its agents in place and emits migrants;
//! 3. **exchange** — migrants are routed to the partition owning their new cell;
//! 4. **commit**  — each partition sorts, merges small bands, recomputes cell totals;
//! 5. **reduce**  — integer summaries.
//!
//! Every random draw is keyed by (seed, stream, entity, tick) and every reduction is an
//! integer sum, so the result is identical for any partition count.
//!
//! Partitions are persistent: each owns its agents and reusable buffers across steps,
//! so a step allocates nothing in steady state.
//!
//! Partitions are contiguous cell ranges of near-equal *cost* (agents and cells, see
//! [`AGENT_COST`]), not of equal cell count: population is very uneven and moves over
//! millennia, so every [`BALANCE_EVERY`] steps the world re-splits if one partition has
//! drifted more than [`IMBALANCE_TOLERANCE_PCT`] above the mean. Results never depend
//! on the partitioning, so these are performance settings, not model parameters.

use crate::agents::{Agents, EINKORN, EMMER, SHARE_ONE, Settlement, Traits};
use crate::catchment::Catchments;
use crate::environment::{self, Farmland};
use crate::exchange::Obsidian;
use crate::inputs::Inputs;
use crate::mechanisms::{self, Mechanism, PartitionView, Sense};
use crate::params::Params;
use crate::schedule::Schedule;
use oikos_core::time::TICKS_PER_YEAR;
use oikos_core::{EntityId, Philox4x32, Stream, Tick};
use oikos_grid::Grid;
use serde::{Deserialize, Serialize};
use std::ops::Range;
use std::sync::Arc;

/// Which cells a world simulates: every descendant of one H3 base cell (M0 toy worlds,
/// benchmarks), or the cells listed by a geometry pack (`{"geometry": "<pack id>"}`,
/// e.g. the M1 region; ADR-0040).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum WorldConfig {
    BaseCell(BaseCellRegion),
    Geometry(GeometryRegion),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseCellRegion {
    pub base_cell: u8,
    pub resolution: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryRegion {
    /// Id of the geometry pack listing the region's cells.
    pub geometry: String,
}

impl<'de> Deserialize<'de> for WorldConfig {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<WorldConfig, D::Error> {
        use serde::de::Error;
        let v = serde_json::Value::deserialize(d)?;
        // Pick the form by its key, so a typo gets that form's own error message.
        if v.get("geometry").is_some() {
            serde_json::from_value(v).map(WorldConfig::Geometry).map_err(D::Error::custom)
        } else {
            serde_json::from_value(v).map(WorldConfig::BaseCell).map_err(D::Error::custom)
        }
    }
}

impl WorldConfig {
    /// The M0 default: base cell 22 (Fertile Crescent) at H3 resolution 4.
    pub const SOUTHWEST_ASIA: WorldConfig = WorldConfig::base_cell(22, 4);

    pub const fn base_cell(base_cell: u8, resolution: u8) -> WorldConfig {
        WorldConfig::BaseCell(BaseCellRegion { base_cell, resolution })
    }

    pub fn geometry(pack_id: &str) -> WorldConfig {
        WorldConfig::Geometry(GeometryRegion { geometry: pack_id.to_string() })
    }

    /// The grid, from H3 topology only. A geometry region needs its pack in `inputs`;
    /// forcing packs need a geometry region (their cells are the pack's).
    pub fn build_grid(&self, inputs: &Inputs) -> Result<Grid, String> {
        match self {
            WorldConfig::BaseCell(r) => {
                if !inputs.is_empty() {
                    return Err(
                        "forcing packs need a world defined by its geometry pack ({\"geometry\": \"<id>\"})"
                            .into(),
                    );
                }
                Grid::from_base_cell(r.base_cell, r.resolution).map_err(|e| e.to_string())
            }
            WorldConfig::Geometry(r) => {
                let g = inputs.geometry().ok_or_else(|| {
                    format!("world needs its geometry pack {} among the inputs", r.geometry)
                })?;
                if g.id() != r.geometry {
                    return Err(format!(
                        "world names geometry pack {}, but the inputs' geometry pack is {}",
                        r.geometry,
                        g.id()
                    ));
                }
                let h3 = g.field("h3")?.raw_u64().ok_or("geometry pack: h3 must be u64")?;
                Grid::from_h3(g.header().region.resolution, &h3)
                    .map_err(|e| format!("geometry pack {}: {e}", g.id()))
            }
        }
    }
}

/// A contiguous range of cells and the agents living in them. Cache-line aligned so
/// threads updating neighboring partitions never share a line.
#[derive(Default)]
#[repr(align(128))]
pub(crate) struct Partition {
    pub(crate) cells: Range<u32>,
    /// Agents in canonical (cell, id) order between steps.
    pub(crate) agents: Agents,
    scratch: Agents,
    counts: Vec<u32>,
    /// Migrants leaving this partition, indexed by destination partition.
    outbox: Vec<Agents>,
    stats: StepStats,
    /// Whether a settlement label was made here this step.
    towns: bool,
    /// Daughters that stayed in their settlement this step ([`PartitionView`]).
    stayed: Vec<(u32, u32, u64, Traits)>,
}

pub struct World {
    pub config: WorldConfig,
    pub grid: Arc<Grid>,
    pub seed: u64,
    pub tick: Tick,
    pub params: Params,
    /// Carrying capacity per cell (people): foraging plus the bands' cultivation
    /// ([`environment::farmed_capacity`]), as of the last commit.
    pub capacity: Vec<u64>,
    /// Forager carrying capacity per cell: from the climate pack (recomputed every
    /// `capacity_period_ticks`) if there is one, else the toy draw.
    pub forage_capacity: Vec<u64>,
    /// Wild-cereal suitability, farm capacity and climate stress per cell (empty without
    /// a climate pack). Derived from `inputs`, so never checkpointed.
    pub farmland: Farmland,
    /// Per cell, the people-weighted sums of the agents' cultivation effort and of effort
    /// times domestic share (fixed point, see [`SHARE_ONE`]); derived from the agents.
    effort_sum: Vec<u64>,
    crop_sum: Vec<u64>,
    /// Likewise for herding (#37): the people-weighted sums of herding effort and of it
    /// times the herd's domestic share.
    herd_sum: Vec<u64>,
    herd_crop_sum: Vec<u64>,
    /// Per cell, the people-weighted sums of effort times emmer's and einkorn's domestic
    /// shares (#38), for the seed bands exchange (#48).
    wheat_sum: Vec<[u64; 2]>,
    /// Per cell, its people as models of cultivation for learners (ADR-0076), recomputed
    /// before each step's decide phase; empty where learning reads the own cell's effort only.
    models: Vec<(f64, f64)>,
    models_for: (f64, f64),
    /// Soil share per cell (fixed point, see [`SHARE_ONE`]; full at the start): worn down
    /// by fields, re-forming slowly ([`environment::erode`]). Checkpointed.
    pub(crate) soil: Vec<u32>,
    /// Obsidian availability per source group and cell, passed down the line every step
    /// ([`crate::exchange`]); empty without an obsidian pack. Checkpointed.
    pub(crate) obsidian: Obsidian,
    /// Next birth counter per cell, used for partition-independent ids.
    pub birth_counter: Vec<u64>,
    /// Population per cell after the last commit.
    pub population: Vec<u64>,
    pub(crate) parts: Vec<Partition>,
    /// Forcing packs (empty for base-cell worlds).
    pub inputs: Arc<Inputs>,
    /// Whether each cell is habitable this step (dry land under the coastline and
    /// hydrology packs; all cells without them). Derived from `inputs` and `tick`, so
    /// never checkpointed.
    pub habitable: Vec<bool>,
    restricted: bool,
    mechanisms: Arc<Vec<Box<dyn Mechanism>>>,
    schedule: Schedule,
    steps_since_balance: u64,
    /// Whether any group may live with others (ADR-0082): set once a settlement label is
    /// made; without, every group lives alone and mechanisms skip looking for settlements.
    towns: bool,
    /// Reused buffers for re-partitioning (per-cell cost, all agents in order).
    cell_costs: Vec<u64>,
    gathered: Agents,
    /// Each cell's catchment (#50), from the grid, the geometry pack and the parameters.
    catchments: Arc<Catchments>,
    /// What mechanisms see over catchments ([`Reach`]).
    reach: Reach,
}

/// What mechanisms see over catchments where they reach beyond cells (#50, [`Sense`]):
/// derived from the forcing, the per-cell sums and the agents, so never checkpointed.
/// Empty while every cell is its own catchment.
#[derive(Default)]
struct Reach {
    /// Each cell's own forager capacity before rounding.
    forage_exact: Vec<f64>,
    /// Over each catchment, what its land feeds by use (with the land's period)...
    forage: Vec<u64>,
    farm: Vec<f64>,
    flood: Vec<f64>,
    herd_range: Vec<f64>,
    herd_pasture: Vec<f64>,
    /// ...and its people, their practice sums and the capacity they farm (every step).
    population: Vec<u64>,
    effort_sum: Vec<u64>,
    crop_sum: Vec<u64>,
    herd_sum: Vec<u64>,
    herd_crop_sum: Vec<u64>,
    wheat_sum: Vec<[u64; 2]>,
    capacity: Vec<u64>,
    /// Each cell's largest settlement.
    largest: Vec<Settlement>,
    /// Per catchment: its people, practice and wheat sums, and capacity (reused buffers).
    sums: Vec<[u64; 7]>,
    block_capacity: Vec<u64>,
}

/// Relative step cost of one agent and of one cell (empty or not: sorting and recounting
/// touch every cell). Measured on the M0 toy, single thread: ~22 ns per agent-step,
/// ~2.6 ns per empty cell-step.
pub const AGENT_COST: u64 = 8;
pub const CELL_COST: u64 = 1;
/// Steps between balance checks.
pub const BALANCE_EVERY: u64 = 16;
/// Re-split when the costliest partition exceeds the mean by more than this.
pub const IMBALANCE_TOLERANCE_PCT: u64 = 10;
/// Recorded shares are parts per this ([`World::recorded_fields`]).
pub const FIELD_ONE: u64 = 10_000;

/// Exact accounting for one step (used by conservation checks).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepStats {
    pub births: u64,
    pub deaths: u64,
    pub fissions: u64,
    pub migrants: u64,
}

impl StepStats {
    pub fn add(&mut self, o: StepStats) {
        self.births += o.births;
        self.deaths += o.deaths;
        self.fissions += o.fissions;
        self.migrants += o.migrants;
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub tick: u64,
    pub calendar_year: i64,
    pub population: u64,
    pub agents: u64,
    pub occupied_cells: u64,
}

impl World {
    /// A fresh world without forcing packs: environment and initial bands drawn from
    /// the seed.
    pub fn new(config: WorldConfig, seed: u64, params: Params) -> Result<World, String> {
        World::with_inputs(config, seed, params, Inputs::none())
    }

    /// [`World::new`] with forcing packs (a geometry-pack region and its fields).
    pub fn with_inputs(
        config: WorldConfig,
        seed: u64,
        params: Params,
        inputs: impl Into<Arc<Inputs>>,
    ) -> Result<World, String> {
        params.validate()?;
        let inputs = inputs.into();
        let grid = Arc::new(config.build_grid(&inputs)?);
        let n = grid.len();
        // Forager capacity from the climate if there is a climate pack, else the toy draw.
        let mut capacity = vec![0; n];
        let reach = Catchments::of_inputs(&grid, &inputs, params.catchment_radius_km);
        let start = Tick::EPOCH.calendar_year();
        let climate = environment::forage_into(&inputs, start, &params, &reach, None, &mut capacity);
        if !climate {
            let env = Philox4x32::new(seed, Stream::Environment);
            let span = params.capacity_max - params.capacity_min + 1;
            for (c, k) in capacity.iter_mut().enumerate() {
                let b = env.block(c as u64, 0, 0);
                *k = params.capacity_min + ((b[0] as u64) << 32 | b[1] as u64) % span;
            }
        }

        let mut birth_counter = vec![0; n];
        let mut agents = Agents::default();
        let foragers = mechanisms::Inequality::foragers(&params);
        // Without a climate (or with zero occupancy), initial bands cluster around one
        // seed-chosen habitable cell and its habitable neighbors.
        let mut habitable = vec![true; n];
        inputs.habitable_into(Tick::EPOCH.calendar_year(), &mut habitable);
        let land: Vec<u32> = (0..n as u32).filter(|&c| habitable[c as usize]).collect();
        if land.is_empty() {
            return Err(format!(
                "no habitable cell in {}: the forcing packs leave no dry land at the start",
                Tick::EPOCH.calendar_year()
            ));
        }
        if climate && params.initial_occupancy > 0.0 {
            // Foragers already live across a region with a climate: one band per habitable
            // cell at `initial_occupancy` of its forager capacity.
            for &cell in &land {
                let w = (params.initial_occupancy * capacity[cell as usize] as f64) as u64;
                if w > 0 {
                    let id = next_id(&mut birth_counter, cell);
                    agents.push_with(id, cell, w, foragers);
                }
            }
            return Ok(World::assemble(
                config,
                grid,
                inputs,
                seed,
                Tick::EPOCH,
                params,
                capacity,
                birth_counter,
                agents,
            ));
        }
        let init = Philox4x32::new(seed, Stream::Initialization);
        let center = land[init.below(0, 0, 0, 0, land.len() as u32) as usize];
        let mut home: Vec<u32> = vec![center];
        home.extend(grid.neighbors(center).iter().filter(|&&c| habitable[c as usize]));
        for b in 0..params.initial_bands {
            let cell = home[init.below(1, 0, b, 0, home.len() as u32) as usize];
            let id = next_id(&mut birth_counter, cell);
            agents.push_with(id, cell, params.initial_weight, foragers);
        }
        agents.sort_canonical();
        Ok(World::assemble(config, grid, inputs, seed, Tick::EPOCH, params, capacity, birth_counter, agents))
    }

    /// A world with `bands_per_cell` bands in every cell, at carrying capacity.
    /// Used by benchmarks and scaling tests.
    pub fn filled(
        config: WorldConfig,
        seed: u64,
        params: Params,
        bands_per_cell: u32,
    ) -> Result<World, String> {
        let n = config.build_grid(&Inputs::none())?.len() as u32;
        World::filled_cells(config, seed, params, bands_per_cell, 0..n)
    }

    /// [`World::filled`], but only `cells` are occupied: a hotspot for load-balancing
    /// benchmarks and tests.
    pub fn filled_cells(
        config: WorldConfig,
        seed: u64,
        params: Params,
        bands_per_cell: u32,
        cells: Range<u32>,
    ) -> Result<World, String> {
        let w = World::new(config, seed, params)?;
        if cells.end as usize > w.grid.len() {
            return Err(format!("cells {cells:?} exceed the grid's {} cells", w.grid.len()));
        }
        let mut birth_counter = vec![0; w.grid.len()];
        let mut agents = Agents::default();
        let bands = bands_per_cell.max(1) as u64;
        let foragers = mechanisms::Inequality::foragers(&w.params);
        for cell in cells {
            let per_band = (w.forage_capacity[cell as usize] / bands).max(1);
            for _ in 0..bands {
                let id = next_id(&mut birth_counter, cell);
                agents.push_with(id, cell, per_band, foragers);
            }
        }
        Ok(World::assemble(
            w.config,
            w.grid,
            w.inputs,
            w.seed,
            w.tick,
            w.params,
            w.forage_capacity,
            birth_counter,
            agents,
        ))
    }

    /// Builds a world from canonical-order agents (one partition) and forager capacity.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn assemble(
        config: WorldConfig,
        grid: Arc<Grid>,
        inputs: Arc<Inputs>,
        seed: u64,
        tick: Tick,
        params: Params,
        forage_capacity: Vec<u64>,
        birth_counter: Vec<u64>,
        agents: Agents,
    ) -> World {
        let n = grid.len();
        let mut population = vec![0; n];
        for (&c, &w) in agents.cell.iter().zip(&agents.weight) {
            population[c as usize] += w;
        }
        let (mut effort_sum, mut crop_sum, mut herd_sum, mut herd_crop_sum) =
            (vec![0; n], vec![0; n], vec![0; n], vec![0; n]);
        let mut wheat_sum = vec![[0; 2]; n];
        let sums = (&mut effort_sum[..], &mut crop_sum[..], &mut herd_sum[..], &mut herd_crop_sum[..]);
        add_farming(&agents, 0, sums, &mut wheat_sum);
        let part = Partition { cells: 0..n as u32, agents, ..Partition::default() };
        let mut habitable = vec![true; n];
        let restricted = inputs.habitable_into(tick.calendar_year(), &mut habitable);
        let farmland = Farmland::new(&inputs, land_year(tick, &params), &params, n);
        let obsidian = Obsidian::new(&inputs, n);
        let mechanisms = mechanisms::defaults(&params);
        let schedule = schedule_of(&mechanisms).expect("default mechanisms form a valid schedule");
        let mut w = World {
            config,
            grid,
            seed,
            tick,
            params,
            capacity: forage_capacity.clone(),
            forage_capacity,
            farmland,
            effort_sum,
            crop_sum,
            herd_sum,
            herd_crop_sum,
            wheat_sum,
            models: Vec::new(),
            models_for: (0.0, 0.0),
            soil: vec![SHARE_ONE; n],
            obsidian,
            birth_counter,
            population,
            parts: vec![part],
            inputs,
            habitable,
            restricted,
            mechanisms: Arc::new(mechanisms),
            schedule,
            steps_since_balance: 0,
            towns: false,
            cell_costs: Vec::new(),
            gathered: Agents::default(),
            catchments: Arc::new(Catchments::default()),
            reach: Reach::default(),
        };
        let towns = w.agent_slices().any(|a| !a.alone(0..a.len()));
        w.towns = towns;
        w.set_catchments();
        w.farm_all();
        w
    }

    /// Builds the catchments for the parameters and, where they reach beyond cells, the
    /// land over them.
    fn set_catchments(&mut self) {
        self.catchments =
            Arc::new(Catchments::of_inputs(&self.grid, &self.inputs, self.params.catchment_radius_km));
        self.reach = Reach::default();
        if self.catchments.is_trivial() {
            return;
        }
        // Each cell's own forager capacity before rounding, leaving the world's (restored
        // from a checkpoint, perhaps of the period before) as it is.
        let n = self.grid.len();
        let (year, mut rounded) = (land_year(self.tick, &self.params), vec![0; n]);
        self.reach.forage_exact.resize(n, 0.0);
        let exact = Some(&mut self.reach.forage_exact[..]);
        if !environment::forage_into(&self.inputs, year, &self.params, &self.catchments, exact, &mut rounded)
        {
            let toy = self.forage_capacity.iter().map(|&k| k as f64);
            self.reach.forage_exact.splice(.., toy);
        }
        self.reach_land();
    }

    /// Recomputes forager capacity for the year of the last capacity period start, with
    /// water within each catchment, and before rounding where catchments reach beyond cells.
    /// Without a climate pack it stays as it is.
    fn forage(&mut self) {
        let year = land_year(self.tick, &self.params);
        let n = self.grid.len();
        let exact = if self.catchments.is_trivial() {
            None
        } else {
            self.reach.forage_exact.resize(n, 0.0);
            Some(&mut self.reach.forage_exact[..])
        };
        let (inputs, p) = (&self.inputs, &self.params);
        let climate =
            environment::forage_into(inputs, year, p, &self.catchments, exact, &mut self.forage_capacity);
        if !climate && !self.catchments.is_trivial() {
            let toy = self.forage_capacity.iter().map(|&k| k as f64);
            self.reach.forage_exact.splice(.., toy);
        }
    }

    /// What the land of each catchment feeds, by use: its cells' sums, seen by each of them.
    fn reach_land(&mut self) {
        let c = &*self.catchments;
        let over = |x: &[f64]| -> Vec<f64> {
            if x.is_empty() {
                return Vec::new();
            }
            let sums: Vec<f64> =
                (0..c.len()).map(|b| c.members(b).iter().map(|&m| x[m as usize]).sum()).collect();
            (0..x.len()).map(|i| sums[c.of(i)]).collect()
        };
        let (r, f) = (&mut self.reach, &self.farmland);
        r.forage = over(&r.forage_exact).into_iter().map(|k| (k as u64).max(1)).collect();
        (r.farm, r.flood, r.herd_range, r.herd_pasture) =
            (over(&f.farm), over(&f.flood), over(&f.herd_range), over(&f.herd_pasture));
    }

    /// The people of each catchment, their practice sums and the capacity they farm, seen
    /// by each of its cells, and each cell's largest settlement, from the last commit.
    fn reach_people(&mut self, ranges: &[Range<u32>]) {
        let (n, c) = (self.grid.len(), &*self.catchments);
        let r = &mut self.reach;
        // People, effort, crop, herding, herd crop and the two wheats per catchment.
        r.sums.clear();
        r.sums.resize(c.len(), [0; 7]);
        for cell in 0..n {
            let s = &mut r.sums[c.of(cell)];
            let [emmer, einkorn] = self.wheat_sum[cell];
            let own = [
                self.population[cell],
                self.effort_sum[cell],
                self.crop_sum[cell],
                self.herd_sum[cell],
                self.herd_crop_sum[cell],
                emmer,
                einkorn,
            ];
            for (a, x) in s.iter_mut().zip(own) {
                *a += x;
            }
        }
        let one = SHARE_ONE as f64;
        let farming = self.farmland.active();
        r.block_capacity.clear();
        r.block_capacity.extend((0..c.len()).map(|b| {
            let [people, effort, crop, herd, herd_crop, ..] = r.sums[b];
            let at = c.members(b)[0] as usize; // the catchment's land, as each cell sees it
            if !farming || (effort == 0 && herd == 0) {
                return r.forage[at];
            }
            let people = people as f64 * one;
            let land = environment::Land {
                forage: r.forage[at],
                farm: r.farm[at],
                flood: r.flood[at],
                herd_range: r.herd_range[at],
                herd_pasture: r.herd_pasture[at],
            };
            let practice = environment::Practice {
                effort: effort as f64 / people,
                crop: crop as f64 / people,
                herding: herd as f64 / people,
                herd_crop: herd_crop as f64 / people,
            };
            environment::capacity_of(&land, &practice, &self.params)
        }));
        for v in
            [&mut r.population, &mut r.effort_sum, &mut r.crop_sum, &mut r.herd_sum, &mut r.herd_crop_sum]
        {
            v.resize(n, 0);
        }
        r.wheat_sum.resize(n, [0; 2]);
        r.capacity.resize(n, 0);
        for cell in 0..n {
            let b = c.of(cell);
            let [people, effort, crop, herd, herd_crop, emmer, einkorn] = r.sums[b];
            (
                r.population[cell],
                r.effort_sum[cell],
                r.crop_sum[cell],
                r.herd_sum[cell],
                r.herd_crop_sum[cell],
            ) = (people, effort, crop, herd, herd_crop);
            r.wheat_sum[cell] = [emmer, einkorn];
            r.capacity[cell] = r.block_capacity[b];
        }
        r.largest.resize(n, Settlement::NONE);
        let items: Vec<_> = self.parts.iter().zip(split_by_ranges(&mut r.largest, ranges)).collect();
        par_for_each(items, |(part, largest)| part.agents.largest_settlements(part.cells.clone(), largest));
    }

    /// Each cell's people as models of cultivation, once per step rather than once for
    /// every learner that sees them ([`mechanisms::Cultivation::cell_models`]).
    fn update_models(&mut self) {
        let learning = mechanisms::Cultivation::from_params(&self.params);
        if !self.farmland.active() || learning.own_cell_only() {
            self.models.clear();
            return;
        }
        self.models.resize(self.grid.len(), (0.0, 0.0));
        self.models_for = (learning.success_bias, learning.wild_yield_ratio);
        // Per catchment where cells are not their own, as mechanisms see them.
        let (r, f) = (&self.reach, &self.farmland);
        let (forage, people, effort, crop, farm, flood) = if self.catchments.is_trivial() {
            (&self.forage_capacity, &self.population, &self.effort_sum, &self.crop_sum, &f.farm, &f.flood)
        } else {
            (&r.forage, &r.population, &r.effort_sum, &r.crop_sum, &r.farm, &r.flood)
        };
        for (c, m) in self.models.iter_mut().enumerate() {
            *m = if effort[c] == 0 {
                (0.0, people[c] as f64) // foragers only: no payoff to compute
            } else {
                learning.cell_models((farm[c], flood[c]), forage[c], people[c], effort[c], crop[c])
            };
        }
    }

    /// Recomputes every cell's capacity from forager capacity, farmland and the agents'
    /// cultivation totals.
    fn farm_all(&mut self) {
        if !self.farmland.active() {
            self.capacity.copy_from_slice(&self.forage_capacity);
            return;
        }
        let n = self.grid.len();
        let land = (&self.forage_capacity[..], &self.farmland);
        let sums = (&self.effort_sum[..], &self.crop_sum[..], &self.herd_sum[..], &self.herd_crop_sum[..]);
        farm_cells(land, 0..n as u32, &self.population, sums, &mut self.capacity, &self.params);
    }

    /// Recomputes forager capacity and farmland for the year of the last capacity period
    /// start, then every cell's capacity.
    fn refresh_land(&mut self) {
        let year = land_year(self.tick, &self.params);
        self.forage();
        self.farmland.update(&self.inputs, year, &self.params, self.grid.len());
        if self.farmland.active() {
            for (farm, &s) in self.farmland.farm.iter_mut().zip(&self.soil) {
                if s < SHARE_ONE {
                    *farm *= environment::soil_yield(s as f64 / SHARE_ONE as f64, &self.params);
                }
            }
        }
        if !self.catchments.is_trivial() {
            self.reach_land();
        }
        self.farm_all();
    }

    /// One capacity period of erosion and soil formation, from each cell's effort at the
    /// last commit. Without a bedrock pack, or data for a cell, its soil is deep.
    fn erode_soil(&mut self) {
        let years = self.params.capacity_period_ticks as f64 / TICKS_PER_YEAR as f64;
        let one = SHARE_ONE as f64;
        let bedrock = self.inputs.field(environment::BEDROCK.0, environment::BEDROCK.1).ok();
        for c in 0..self.grid.len() {
            let effort = match self.population[c] {
                0 => 0.0,
                n => self.effort_sum[c] as f64 / (n as f64 * one),
            };
            let s = self.soil[c];
            if effort > 0.0 || s < SHARE_ONE {
                let depth = bedrock
                    .as_ref()
                    .and_then(|f| f.get(0, c))
                    .map_or(1.0, |d| d.clamp(environment::MIN_DEPTH, 1.0));
                self.soil[c] =
                    Traits::toward(s, environment::erode(s as f64 / one, effort, years, depth, &self.params));
            }
        }
    }

    /// Soil share of each cell (1 = its original erodible soil).
    pub fn soil(&self) -> Vec<f64> {
        self.soil.iter().map(|&s| s as f64 / SHARE_ONE as f64).collect()
    }

    /// Sets each cell's soil (fixed point) and the capacity it supports (restoring).
    pub(crate) fn restore_soil(&mut self, soil: Vec<u32>) {
        assert_eq!(soil.len(), self.grid.len());
        self.soil = soil;
        self.refresh_land();
    }

    /// Obsidian source groups (the obsidian pack's fields; none without one).
    pub fn obsidian_sources(&self) -> &[String] {
        &self.obsidian.sources
    }

    /// Availability of source group `source`'s obsidian in each cell (1 at the source).
    pub fn obsidian(&self, source: usize) -> Vec<f64> {
        self.obsidian.available[source].iter().map(|&x| x as f64 / SHARE_ONE as f64).collect()
    }

    /// Sets each source group's availability per cell (fixed point; restoring).
    pub(crate) fn restore_obsidian(&mut self, available: Vec<Vec<u32>>) {
        assert_eq!(available.len(), self.obsidian.sources.len());
        assert!(available.iter().all(|a| a.len() == self.grid.len()));
        self.obsidian.available = available;
    }

    /// Mean cultivation effort of each cell's people (0 in empty cells).
    pub fn cultivation(&self) -> Vec<f64> {
        let one = SHARE_ONE as f64;
        (0..self.grid.len())
            .map(|c| match self.population[c] {
                0 => 0.0,
                n => self.effort_sum[c] as f64 / (n as f64 * one),
            })
            .collect()
    }

    /// Settlements per cell: each counted as far as it is settled (its groups'
    /// people-weighted cultivation effort, a lone group's own), which is what surveys find
    /// (ADR-0072, ADR-0082). Summed in fixed point, so exact whatever the partitions.
    pub fn settlements(&self) -> Vec<f64> {
        let mut sum = vec![0u64; self.grid.len()];
        self.for_each_settlement(|cell, s| sum[cell as usize] += s.cultivation() as u64);
        sum.into_iter().map(|x| x as f64 / SHARE_ONE as f64).collect()
    }

    /// Village size per cell (ADR-0078): its settlements' log people, each counted as far
    /// as it is settled, as `settlements` counts them; divided by it, the mean log size of
    /// the cell's settlements. Summed in canonical agent order, so the same whatever the
    /// partitions.
    pub fn village_sizes(&self) -> Vec<f64> {
        let mut sum = vec![0.0f64; self.grid.len()];
        self.for_each_settlement(|cell, s| {
            if s.cultivation() > 0 {
                sum[cell as usize] +=
                    s.cultivation() as f64 / SHARE_ONE as f64 * oikos_math::ln(s.people as f64);
            }
        });
        sum
    }

    /// Infected people per cell (ADR-0075, #42): each settlement's people times the endemic
    /// prevalence at its size and pooled practice ([`crate::mechanisms::Disease::prevalence`]),
    /// whether or not the infection kills (`disease_mortality`), as lesions on its dead
    /// would show it. Summed in canonical agent order, so the same whatever the partitions.
    pub fn infected(&self) -> Vec<f64> {
        let disease = crate::mechanisms::Disease::from_params(&self.params);
        let mut sum = vec![0.0f64; self.grid.len()];
        self.for_each_settlement(|cell, s| {
            let people = s.people as f64;
            let effort = s.cultivation() as f64 / SHARE_ONE as f64;
            sum[cell as usize] += people * disease.prevalence(people, effort);
        });
        sum
    }

    /// Calls `f` with each settlement holding people and its cell, in canonical order.
    fn for_each_settlement(&self, mut f: impl FnMut(u32, &crate::agents::Settlement)) {
        for a in self.agent_slices() {
            let mut i = 0;
            while i < a.len() {
                let cell = a.cell[i];
                let end = i + a.cell[i..].iter().take_while(|&&c| c == cell).count();
                a.for_each_settlement(i..end, |s| f(cell, s));
                i = end;
            }
        }
    }

    /// Wealth inequality of each cell's settled people (ADR-0074): the variance of log
    /// household wealth of its groups, weighted by their settled people (people times
    /// cultivation effort), as excavated houses of a village would show it; 0 where nobody
    /// farms. Sums are exact integers, so the result is the same on every host.
    pub fn inequality(&self) -> Vec<f64> {
        let n = self.grid.len();
        let (mut settled, mut sum) = (vec![0u128; n], vec![0u128; n]);
        for a in self.agent_slices() {
            for i in 0..a.len() {
                let s = a.weight[i] as u128 * a.cultivation[i] as u128;
                settled[a.cell[i] as usize] += s;
                sum[a.cell[i] as usize] += s * a.inequality[i] as u128;
            }
        }
        (0..n)
            .map(|c| match settled[c] {
                0 => 0.0,
                s => {
                    (sum[c] / s) as f64 / SHARE_ONE as f64 + (sum[c] % s) as f64 / s as f64 / SHARE_ONE as f64
                }
            })
            .collect()
    }

    /// Domestic share of each cell's cultivated crop: the effort-weighted mean of its
    /// bands' domestication (0 where nobody cultivates), as archaeobotanical assemblages
    /// would show it.
    pub fn domestication(&self) -> Vec<f64> {
        (0..self.grid.len())
            .map(|c| match self.effort_sum[c] {
                0 => 0.0,
                e => self.crop_sum[c] as f64 / e as f64,
            })
            .collect()
    }

    /// Domestic share of the emmer each cell sows with its crop (#38), effort-weighted as
    /// [`World::domestication`] is (0 where nobody cultivates).
    pub fn emmer_domestication(&self) -> Vec<f64> {
        self.wheat_domestication(EMMER)
    }

    /// Domestic share of the einkorn each cell sows with its crop (#38).
    pub fn einkorn_domestication(&self) -> Vec<f64> {
        self.wheat_domestication(EINKORN)
    }

    /// Summed from the agents when asked (records sample it once a decade), with each
    /// group's effort times share truncated as [`Traits::practice`] truncates the crop's.
    fn wheat_domestication(&self, wheat: usize) -> Vec<f64> {
        let one = SHARE_ONE as u64;
        let mut sum = vec![0u64; self.grid.len()];
        for a in self.agent_slices() {
            for i in (0..a.len()).filter(|&i| a.cultivation[i] != 0) {
                let d = a.wheat_domestication[i][wheat] as u64;
                sum[a.cell[i] as usize] += a.weight[i] * (a.cultivation[i] as u64 * d / one);
            }
        }
        (0..self.grid.len())
            .map(|c| match self.effort_sum[c] {
                0 => 0.0,
                e => sum[c] as f64 / e as f64,
            })
            .collect()
    }

    /// Mean herding effort of each cell's people (#37), as `cultivation` is of theirs.
    pub fn herding(&self) -> Vec<f64> {
        let one = SHARE_ONE as f64;
        (0..self.grid.len())
            .map(|c| match self.population[c] {
                0 => 0.0,
                n => self.herd_sum[c] as f64 / (n as f64 * one),
            })
            .collect()
    }

    /// Domestic share of each cell's managed herds, herding-effort-weighted (#37); 0 where
    /// nobody herds.
    pub fn herd_domestication(&self) -> Vec<f64> {
        (0..self.grid.len())
            .map(|c| match self.herd_sum[c] {
                0 => 0.0,
                h => self.herd_crop_sum[c] as f64 / h as f64,
            })
            .collect()
    }

    /// Per-cell shares that run outputs record, in parts per [`FIELD_ONE`] (truncated,
    /// from the fixed-point state, so exact on every platform): cultivation, domestication
    /// and soil; dry land where packs restrict it; each obsidian source group's
    /// availability as `obsidian_<group>`.
    pub fn recorded_fields(&self) -> Vec<(String, Vec<u64>)> {
        let one = SHARE_ONE as u128;
        let part = |num: u128, den: u128| (num * FIELD_ONE as u128).checked_div(den).unwrap_or(0) as u64;
        let n = self.grid.len();
        let mut fields = vec![
            (
                "cultivation".into(),
                (0..n).map(|c| part(self.effort_sum[c] as u128, self.population[c] as u128 * one)).collect(),
            ),
            (
                "domestication".into(),
                (0..n).map(|c| part(self.crop_sum[c] as u128, self.effort_sum[c] as u128)).collect(),
            ),
            ("soil".into(), self.soil.iter().map(|&s| part(s as u128, one)).collect()),
        ];
        let surface = environment::Surface::new(&self.inputs, land_year(self.tick, &self.params));
        if surface.restricted() {
            let dry = (0..n).map(|c| (surface.dry(c) * FIELD_ONE as f64).round() as u64).collect();
            fields.push(("land".into(), dry));
        }
        for (name, available) in self.obsidian.sources.iter().zip(&self.obsidian.available) {
            fields.push((
                format!("obsidian_{name}"),
                available.iter().map(|&x| part(x as u128, one)).collect(),
            ));
        }
        fields
    }

    /// Replaces the mechanisms; fails if their declared accesses are ambiguous.
    pub fn set_mechanisms(&mut self, mechanisms: Vec<Box<dyn Mechanism>>) -> Result<(), String> {
        self.schedule = schedule_of(&mechanisms)?;
        self.mechanisms = Arc::new(mechanisms);
        Ok(())
    }

    /// Replaces the parameters and rebuilds the default mechanisms from them.
    pub fn set_params(&mut self, params: Params) -> Result<(), String> {
        params.validate()?;
        self.set_mechanisms(mechanisms::defaults(&params))?;
        self.params = params;
        // Catchments and capacity derived from forcing follow the new parameters at once.
        self.set_catchments();
        self.refresh_land();
        Ok(())
    }

    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    pub fn partitions(&self) -> usize {
        self.parts.len()
    }

    /// Re-splits agents into `n` contiguous cell ranges of near-equal cost (no-op if
    /// already `n`).
    pub fn set_partitions(&mut self, n: usize) {
        let n = n.max(1);
        if n != self.parts.len() {
            self.repartition(n);
        }
    }

    /// The cell range of each partition, in order.
    pub fn partition_ranges(&self) -> Vec<Range<u32>> {
        self.parts.iter().map(|p| p.cells.clone()).collect()
    }

    /// Estimated step cost of each partition ([`AGENT_COST`], [`CELL_COST`]).
    pub fn partition_costs(&self) -> Vec<u64> {
        self.parts.iter().map(partition_cost).collect()
    }

    /// Uses `n` partitions; every [`BALANCE_EVERY`] steps re-splits if they drifted
    /// out of balance.
    fn balance(&mut self, n: usize) {
        let n = n.max(1);
        self.steps_since_balance += 1;
        if n != self.parts.len() {
            return self.repartition(n);
        }
        if n == 1 || self.steps_since_balance < BALANCE_EVERY {
            return;
        }
        self.steps_since_balance = 0;
        let (max, total) = self.parts.iter().map(partition_cost).fold((0, 0), |(m, t), c| (m.max(c), t + c));
        if max * n as u64 * 100 > total * (100 + IMBALANCE_TOLERANCE_PCT) {
            self.repartition(n);
        }
    }

    /// Splits cells into `n` ranges of near-equal cost and moves agents to match,
    /// reusing every buffer (no allocation once capacities have grown).
    fn repartition(&mut self, n: usize) {
        self.steps_since_balance = 0;
        let costs = &mut self.cell_costs;
        costs.clear();
        costs.resize(self.grid.len(), CELL_COST);
        for p in &self.parts {
            for &c in &p.agents.cell {
                costs[c as usize] += AGENT_COST;
            }
        }
        let ranges = oikos_grid::partition_weighted(costs, n);
        if ranges.iter().eq(self.parts.iter().map(|p| &p.cells)) {
            return;
        }
        let all = &mut self.gathered;
        all.clear();
        for p in &mut self.parts {
            all.append(&mut p.agents);
        }
        self.parts.resize_with(n, Partition::default);
        for (p, cells) in self.parts.iter_mut().zip(ranges) {
            p.agents.extend_from(all, all.cell_range(cells.clone()));
            p.cells = cells;
        }
    }

    /// Agents of every partition, in canonical global order.
    pub fn agent_slices(&self) -> impl Iterator<Item = &Agents> {
        self.parts.iter().map(|p| &p.agents)
    }

    pub fn agent_count(&self) -> usize {
        self.parts.iter().map(|p| p.agents.len()).sum()
    }

    pub fn total_agent_weight(&self) -> u64 {
        self.parts.iter().map(|p| p.agents.total_weight()).sum()
    }

    pub fn total_population(&self) -> u64 {
        self.population.iter().sum()
    }

    pub fn summary(&self) -> Summary {
        Summary {
            tick: self.tick.0,
            calendar_year: self.tick.calendar_year(),
            population: self.total_population(),
            agents: self.agent_count() as u64,
            occupied_cells: self.population.iter().filter(|&&p| p > 0).count() as u64,
        }
    }

    /// Advances one base period (gcd of mechanism periods) using `parts` spatial partitions.
    pub fn step(&mut self, parts: usize) -> StepStats {
        self.balance(parts);
        let ranges: Vec<Range<u32>> = self.parts.iter().map(|p| p.cells.clone()).collect();
        let nparts = ranges.len();

        if self.restricted {
            self.inputs.habitable_into(self.tick.calendar_year(), &mut self.habitable);
        }
        if self.tick.0 % self.params.capacity_period_ticks == 0 && self.farmland.active() {
            self.erode_soil();
            self.refresh_land();
        }
        if self.obsidian.active() {
            self.obsidian.exchange(&self.grid, &self.catchments, &self.population, self.params.obsidian_loss);
        }
        let reach = !self.catchments.is_trivial();
        if reach {
            self.reach_people(&ranges);
        }
        self.update_models();

        // Sense + decide (parallel over partitions), running due mechanisms in order.
        {
            let (r, f) = (&self.reach, &self.farmland);
            let sense = Sense {
                grid: &self.grid,
                inputs: &self.inputs,
                habitable: &self.habitable,
                restricted: self.restricted,
                capacity: if reach { &r.capacity } else { &self.capacity },
                forage_capacity: if reach { &r.forage } else { &self.forage_capacity },
                farming: f.active(),
                suitability: &f.suitability,
                climate_stress: &f.stress,
                climate_variability: &f.variability,
                harvest_cv: &f.harvest_cv,
                population: if reach { &r.population } else { &self.population },
                effort_sum: if reach { &r.effort_sum } else { &self.effort_sum },
                farm: if reach { &r.farm } else { &f.farm },
                floodplain: &f.floodplain,
                flood: if reach { &r.flood } else { &f.flood },
                range: &f.range,
                pasture: &f.pasture,
                range_feeds: if reach { &r.herd_range } else { &f.herd_range },
                pasture_feeds: if reach { &r.herd_pasture } else { &f.herd_pasture },
                herd_sum: if reach { &r.herd_sum } else { &self.herd_sum },
                herd_crop_sum: if reach { &r.herd_crop_sum } else { &self.herd_crop_sum },
                crop_sum: if reach { &r.crop_sum } else { &self.crop_sum },
                wheat_sum: if reach { &r.wheat_sum } else { &self.wheat_sum },
                models: &self.models,
                models_for: self.models_for,
                catchments: &self.catchments,
                cell_population: &self.population,
                largest: &r.largest,
                towns: self.towns,
                tick: self.tick,
                seed: self.seed,
            };
            let due: Vec<&dyn Mechanism> =
                self.schedule.due(self.tick.0).map(|i| self.mechanisms[i].as_ref()).collect();
            let counters = split_by_ranges(&mut self.birth_counter, &ranges);
            let items: Vec<_> = self.parts.iter_mut().zip(counters).collect();
            par_for_each(items, |(part, counters)| {
                part.outbox.resize_with(ranges.len(), Agents::default);
                part.outbox.iter_mut().for_each(Agents::clear);
                part.stats = StepStats::default();
                part.stayed.clear();
                let mut view = PartitionView {
                    cells: part.cells.clone(),
                    agents: &mut part.agents,
                    stats: &mut part.stats,
                    counters,
                    outbox: &mut part.outbox,
                    ranges: &ranges,
                    towns: &mut part.towns,
                    stayed: &mut part.stayed,
                };
                // Tiles end on cell boundaries; agents spawned during the step lie past `n`.
                let n = view.agents.len();
                let mut start = 0;
                while start < n {
                    let mut end = (start + TILE_AGENTS).min(n);
                    while end < n && view.agents.cell[end] == view.agents.cell[end - 1] {
                        end += 1;
                    }
                    for m in &due {
                        m.decide(&sense, &mut view, start..end);
                    }
                    start = end;
                }
            });
        }

        // Exchange: move outboxes to their destination partitions (buffers keep capacity).
        for src in 0..nparts {
            for dst in 0..nparts {
                if self.parts[src].outbox[dst].is_empty() {
                    continue;
                }
                let mut moving = std::mem::take(&mut self.parts[src].outbox[dst]);
                self.parts[dst].agents.append(&mut moving);
                self.parts[src].outbox[dst] = moving;
            }
        }

        // Commit (parallel): canonical sort, merge small bands, recount population and,
        // where the land supports cultivation, the capacity the bands farm.
        let threshold = self.params.merge_threshold;
        let farming = self.farmland.active();
        let land = (&self.forage_capacity[..], &self.farmland);
        let params = &self.params;
        let pops = split_by_ranges(&mut self.population, &ranges);
        let efforts = split_by_ranges(&mut self.effort_sum, &ranges);
        let crops = split_by_ranges(&mut self.crop_sum, &ranges);
        let herds = split_by_ranges(&mut self.herd_sum, &ranges);
        let herd_crops = split_by_ranges(&mut self.herd_crop_sum, &ranges);
        let wheats = split_by_ranges(&mut self.wheat_sum, &ranges);
        let caps = split_by_ranges(&mut self.capacity, &ranges);
        let sums = efforts.into_iter().zip(crops).zip(herds.into_iter().zip(herd_crops));
        let items: Vec<_> =
            self.parts.iter_mut().zip(pops).zip(sums).zip(caps.into_iter().zip(wheats)).collect();
        par_for_each(items, |(((part, pop), ((effort, crop), (herd, herd_crop))), (cap, wheat))| {
            let cells = part.cells.clone();
            part.agents.sort_in_cells(cells.clone(), &mut part.scratch, &mut part.counts);
            part.agents.merge_small_into(threshold, &mut part.scratch, cells.start, pop);
            std::mem::swap(&mut part.agents, &mut part.scratch);
            if farming {
                for s in [&mut *effort, &mut *crop, &mut *herd, &mut *herd_crop] {
                    s.fill(0);
                }
                wheat.fill([0; 2]);
                add_farming(&part.agents, cells.start, (effort, crop, herd, herd_crop), wheat);
                farm_cells(land, cells, pop, (effort, crop, herd, herd_crop), cap, params);
            }
        });

        if self.obsidian.active() {
            self.obsidian.abandon(&self.population);
        }

        let mut stats = StepStats::default();
        for p in &self.parts {
            stats.add(p.stats);
            self.towns |= p.towns;
        }
        self.tick = self.tick.advance(self.schedule.base_period());
        stats
    }

    /// Runs `years` simulated years, recording a summary every `summary_every` years.
    pub fn run_years(&mut self, years: u64, parts: usize, summary_every: u64) -> Vec<Summary> {
        self.run_years_observed(years, parts, summary_every, &mut |_, _| Ok(())).expect("no-op observer")
    }

    /// Ticks per step: the schedule's base period.
    pub fn base_period_ticks(&self) -> u64 {
        self.schedule.base_period()
    }

    /// [`World::run_years`], calling `observe` after every step (outputs, diagnostics).
    /// Observers get read-only access, so they cannot change the simulation.
    pub fn run_years_observed(
        &mut self,
        years: u64,
        parts: usize,
        summary_every: u64,
        observe: &mut dyn FnMut(&World, StepStats) -> Result<(), String>,
    ) -> Result<Vec<Summary>, String> {
        let base = self.schedule.base_period();
        let steps = years * oikos_core::time::TICKS_PER_YEAR / base;
        let every = (summary_every * oikos_core::time::TICKS_PER_YEAR / base).max(1);
        let mut out = vec![self.summary()];
        for s in 1..=steps {
            let stats = self.step(parts);
            observe(self, stats)?;
            if s % every == 0 {
                out.push(self.summary());
            }
        }
        Ok(out)
    }
}

/// The year whose land a world uses at `tick`: the start of the current capacity period.
fn land_year(tick: Tick, p: &Params) -> i64 {
    Tick(tick.0 - tick.0 % p.capacity_period_ticks).calendar_year()
}

/// Adds each agent's people-weighted effort and effort times domestic share to the sums
/// of its cell (indexed by `cell - base`).
fn add_farming(
    agents: &Agents,
    base: u32,
    sums: (&mut [u64], &mut [u64], &mut [u64], &mut [u64]),
    wheats: &mut [[u64; 2]],
) {
    let (effort, crop, herd, herd_crop) = sums;
    for i in 0..agents.len() {
        if agents.cultivation[i] == 0 && agents.herding[i] == 0 {
            continue;
        }
        let c = (agents.cell[i] - base) as usize;
        let w = agents.weight[i];
        let [e, d, h, hd] = agents.traits(i).practice();
        effort[c] += w * e;
        crop[c] += w * d;
        herd[c] += w * h;
        herd_crop[c] += w * hd;
        let wheat = &mut wheats[c];
        for (s, x) in wheat.iter_mut().zip(agents.traits(i).wheat_practice()) {
            *s += w * x;
        }
    }
}

/// Capacity of `cells` (local slices indexed by `cell - cells.start`; `land` is the
/// world's farmland, with its forager capacity) from their people's cultivation and
/// herding sums.
fn farm_cells(
    land: (&[u64], &Farmland),
    cells: Range<u32>,
    pop: &[u64],
    sums: (&[u64], &[u64], &[u64], &[u64]),
    cap: &mut [u64],
    p: &Params,
) {
    let one = SHARE_ONE as f64;
    let (forage, f) = land;
    for (i, c) in cells.map(|c| c as usize).enumerate() {
        cap[i] = match (sums.0[i], sums.2[i]) {
            (0, 0) => forage[c],
            (e, h) => {
                let people = pop[i] as f64 * one;
                let land = environment::Land {
                    forage: forage[c],
                    farm: f.farm[c],
                    flood: f.flood[c],
                    herd_range: f.herd_range[c],
                    herd_pasture: f.herd_pasture[c],
                };
                let practice = environment::Practice {
                    effort: e as f64 / people,
                    crop: sums.1[i] as f64 / people,
                    herding: h as f64 / people,
                    herd_crop: sums.3[i] as f64 / people,
                };
                environment::capacity_of(&land, &practice, p)
            }
        };
    }
}

fn partition_cost(p: &Partition) -> u64 {
    CELL_COST * (p.cells.end - p.cells.start) as u64 + AGENT_COST * p.agents.len() as u64
}

fn next_id(counters: &mut [u64], cell: u32) -> EntityId {
    let c = &mut counters[cell as usize];
    let id = EntityId::new(cell, *c);
    *c += 1;
    id
}

/// Disjoint mutable sub-slices of a per-cell array, one per partition range.
fn split_by_ranges<'a, T>(mut data: &'a mut [T], ranges: &[Range<u32>]) -> Vec<&'a mut [T]> {
    let mut out = Vec::with_capacity(ranges.len());
    for r in ranges {
        let (mine, rest) = std::mem::take(&mut data).split_at_mut((r.end - r.start) as usize);
        out.push(mine);
        data = rest;
    }
    out
}

/// Agents per decide tile: a few tens of KB of hot state, comfortably cache-resident.
const TILE_AGENTS: usize = 2048;

fn schedule_of(mechanisms: &[Box<dyn Mechanism>]) -> Result<Schedule, String> {
    Schedule::build(&mechanisms.iter().map(|m| m.spec()).collect::<Vec<_>>())
}

/// Runs `f` on every item: on the rayon pool natively, sequentially on wasm.
fn par_for_each<T: Send>(items: Vec<T>, f: impl Fn(T) + Sync + Send) {
    #[cfg(not(target_family = "wasm"))]
    {
        use rayon::prelude::*;
        items.into_par_iter().for_each(f);
    }
    #[cfg(target_family = "wasm")]
    items.into_iter().for_each(f);
}
