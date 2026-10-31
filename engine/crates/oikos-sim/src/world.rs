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

use crate::agents::{Agents, SHARE_ONE, Traits};
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
    /// Reused buffers for re-partitioning (per-cell cost, all agents in order).
    cell_costs: Vec<u64>,
    gathered: Agents,
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
        let climate =
            environment::capacity_into(&inputs, Tick::EPOCH.calendar_year(), &params, &mut capacity);
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
                    agents.push(id, cell, w);
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
            agents.push(id, cell, params.initial_weight);
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
        for cell in cells {
            let per_band = (w.forage_capacity[cell as usize] / bands).max(1);
            for _ in 0..bands {
                let id = next_id(&mut birth_counter, cell);
                agents.push(id, cell, per_band);
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
        let (mut effort_sum, mut crop_sum) = (vec![0; n], vec![0; n]);
        add_farming(&agents, 0, &mut effort_sum, &mut crop_sum);
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
            cell_costs: Vec::new(),
            gathered: Agents::default(),
        };
        w.farm_all();
        w
    }

    /// Recomputes every cell's capacity from forager capacity, farmland and the agents'
    /// cultivation totals.
    fn farm_all(&mut self) {
        if !self.farmland.active() {
            self.capacity.copy_from_slice(&self.forage_capacity);
            return;
        }
        let n = self.grid.len();
        let land = (&self.forage_capacity[..], &self.farmland.farm[..]);
        let sums = (&self.effort_sum[..], &self.crop_sum[..]);
        farm_cells(land, 0..n as u32, &self.population, sums, &mut self.capacity, &self.params);
    }

    /// Recomputes forager capacity and farmland for the year of the last capacity period
    /// start, then every cell's capacity.
    fn refresh_land(&mut self) {
        let year = land_year(self.tick, &self.params);
        environment::capacity_into(&self.inputs, year, &self.params, &mut self.forage_capacity);
        self.farmland.update(&self.inputs, year, &self.params, self.grid.len());
        if self.farmland.active() {
            for (farm, &s) in self.farmland.farm.iter_mut().zip(&self.soil) {
                if s < SHARE_ONE {
                    *farm *= environment::soil_yield(s as f64 / SHARE_ONE as f64, &self.params);
                }
            }
        }
        self.farm_all();
    }

    /// One capacity period of erosion and soil formation, from each cell's effort at the
    /// last commit.
    fn erode_soil(&mut self) {
        let years = self.params.capacity_period_ticks as f64 / TICKS_PER_YEAR as f64;
        let one = SHARE_ONE as f64;
        for c in 0..self.grid.len() {
            let effort = match self.population[c] {
                0 => 0.0,
                n => self.effort_sum[c] as f64 / (n as f64 * one),
            };
            let s = self.soil[c];
            if effort > 0.0 || s < SHARE_ONE {
                self.soil[c] =
                    Traits::toward(s, environment::erode(s as f64 / one, effort, years, &self.params));
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
        // Capacity derived from forcing follows the new parameters at once.
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
            self.obsidian.exchange(&self.grid, &self.population, self.params.obsidian_loss);
        }

        // Sense + decide (parallel over partitions), running due mechanisms in order.
        {
            let sense = Sense {
                grid: &self.grid,
                inputs: &self.inputs,
                habitable: &self.habitable,
                restricted: self.restricted,
                capacity: &self.capacity,
                forage_capacity: &self.forage_capacity,
                farming: self.farmland.active(),
                suitability: &self.farmland.suitability,
                climate_stress: &self.farmland.stress,
                climate_variability: &self.farmland.variability,
                population: &self.population,
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
                let mut view = PartitionView {
                    cells: part.cells.clone(),
                    agents: &mut part.agents,
                    stats: &mut part.stats,
                    counters,
                    outbox: &mut part.outbox,
                    ranges: &ranges,
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
        let land = (&self.forage_capacity[..], &self.farmland.farm[..]);
        let params = &self.params;
        let pops = split_by_ranges(&mut self.population, &ranges);
        let efforts = split_by_ranges(&mut self.effort_sum, &ranges);
        let crops = split_by_ranges(&mut self.crop_sum, &ranges);
        let caps = split_by_ranges(&mut self.capacity, &ranges);
        let items: Vec<_> =
            self.parts.iter_mut().zip(pops).zip(efforts.into_iter().zip(crops)).zip(caps).collect();
        par_for_each(items, |(((part, pop), (effort, crop)), cap)| {
            let cells = part.cells.clone();
            part.agents.sort_in_cells(cells.clone(), &mut part.scratch, &mut part.counts);
            part.agents.merge_small_into(threshold, &mut part.scratch, cells.start, pop);
            std::mem::swap(&mut part.agents, &mut part.scratch);
            if farming {
                effort.fill(0);
                crop.fill(0);
                add_farming(&part.agents, cells.start, effort, crop);
                farm_cells(land, cells, pop, (effort, crop), cap, params);
            }
        });

        let mut stats = StepStats::default();
        for p in &self.parts {
            stats.add(p.stats);
        }
        self.tick = self.tick.advance(self.schedule.base_period());
        stats
    }

    /// Runs `years` simulated years, recording a summary every `summary_every` years.
    pub fn run_years(&mut self, years: u64, parts: usize, summary_every: u64) -> Vec<Summary> {
        self.run_years_observed(years, parts, summary_every, &mut |_, _| Ok(())).expect("no-op observer")
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
fn add_farming(agents: &Agents, base: u32, effort: &mut [u64], crop: &mut [u64]) {
    for i in 0..agents.len() {
        let e = agents.cultivation[i] as u64;
        if e == 0 {
            continue;
        }
        let c = (agents.cell[i] - base) as usize;
        let w = agents.weight[i];
        effort[c] += w * e;
        crop[c] += w * (e * agents.domestication[i] as u64 / SHARE_ONE as u64);
    }
}

/// Capacity of `cells` (local slices indexed by `cell - cells.start`; `land` is the
/// world's forager capacity and farm capacity) from their people's cultivation sums.
fn farm_cells(
    land: (&[u64], &[f64]),
    cells: Range<u32>,
    pop: &[u64],
    sums: (&[u64], &[u64]),
    cap: &mut [u64],
    p: &Params,
) {
    let one = SHARE_ONE as f64;
    for (i, c) in cells.map(|c| c as usize).enumerate() {
        let (forage, farm) = (land.0[c], land.1[c]);
        cap[i] = match sums.0[i] {
            0 => forage,
            e => {
                let people = pop[i] as f64 * one;
                environment::farmed_capacity(forage, farm, e as f64 / people, sums.1[i] as f64 / people, p)
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
