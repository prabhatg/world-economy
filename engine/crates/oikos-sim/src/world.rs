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

use crate::agents::Agents;
use crate::mechanisms::{self, Mechanism, PartitionView, Sense};
use crate::params::Params;
use crate::schedule::Schedule;
use oikos_core::{EntityId, Philox4x32, Stream, Tick};
use oikos_grid::Grid;
use serde::{Deserialize, Serialize};
use std::ops::Range;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldConfig {
    pub base_cell: u8,
    pub resolution: u8,
}

impl WorldConfig {
    /// The M0/M1 default: base cell 22 (Fertile Crescent) at H3 resolution 4.
    pub const SOUTHWEST_ASIA: WorldConfig = WorldConfig { base_cell: 22, resolution: 4 };

    pub fn build_grid(&self) -> Result<Grid, String> {
        Grid::from_base_cell(self.base_cell, self.resolution).map_err(|e| e.to_string())
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
    /// Carrying capacity per cell (people).
    pub capacity: Vec<u64>,
    /// Next birth counter per cell, used for partition-independent ids.
    pub birth_counter: Vec<u64>,
    /// Population per cell after the last commit.
    pub population: Vec<u64>,
    pub(crate) parts: Vec<Partition>,
    mechanisms: Arc<Vec<Box<dyn Mechanism>>>,
    schedule: Schedule,
}

/// Exact accounting for one step (used by conservation checks).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepStats {
    pub births: u64,
    pub deaths: u64,
    pub fissions: u64,
    pub migrants: u64,
}

impl StepStats {
    fn add(&mut self, o: StepStats) {
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
    /// A fresh world: environment and initial bands drawn from the seed.
    pub fn new(config: WorldConfig, seed: u64, params: Params) -> Result<World, String> {
        params.validate()?;
        let grid = Arc::new(config.build_grid()?);
        let n = grid.len();
        let env = Philox4x32::new(seed, Stream::Environment);
        let span = params.capacity_max - params.capacity_min + 1;
        let capacity: Vec<u64> = (0..n as u64)
            .map(|c| {
                let b = env.block(c, 0, 0);
                let r = ((b[0] as u64) << 32 | b[1] as u64) % span;
                params.capacity_min + r
            })
            .collect();

        let mut birth_counter = vec![0; n];
        let mut agents = Agents::default();
        // Initial bands cluster around one seed-chosen cell and its neighbors.
        let init = Philox4x32::new(seed, Stream::Initialization);
        let center = init.below(0, 0, 0, 0, n as u32);
        let mut home: Vec<u32> = vec![center];
        home.extend_from_slice(grid.neighbors(center));
        for b in 0..params.initial_bands {
            let cell = home[init.below(1, 0, b, 0, home.len() as u32) as usize];
            let id = next_id(&mut birth_counter, cell);
            agents.push(id, cell, params.initial_weight);
        }
        agents.sort_canonical();
        Ok(World::assemble(config, grid, seed, Tick::EPOCH, params, capacity, birth_counter, agents))
    }

    /// A world with `bands_per_cell` bands in every cell, at carrying capacity.
    /// Used by benchmarks and scaling tests.
    pub fn filled(
        config: WorldConfig,
        seed: u64,
        params: Params,
        bands_per_cell: u32,
    ) -> Result<World, String> {
        let w = World::new(config, seed, params)?;
        let mut birth_counter = vec![0; w.grid.len()];
        let mut agents = Agents::default();
        let bands = bands_per_cell.max(1) as u64;
        for cell in 0..w.grid.len() as u32 {
            let per_band = (w.capacity[cell as usize] / bands).max(1);
            for _ in 0..bands {
                let id = next_id(&mut birth_counter, cell);
                agents.push(id, cell, per_band);
            }
        }
        Ok(World::assemble(w.config, w.grid, w.seed, w.tick, w.params, w.capacity, birth_counter, agents))
    }

    /// Builds a world from canonical-order agents (one partition).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn assemble(
        config: WorldConfig,
        grid: Arc<Grid>,
        seed: u64,
        tick: Tick,
        params: Params,
        capacity: Vec<u64>,
        birth_counter: Vec<u64>,
        agents: Agents,
    ) -> World {
        let n = grid.len();
        let mut population = vec![0; n];
        for (&c, &w) in agents.cell.iter().zip(&agents.weight) {
            population[c as usize] += w;
        }
        let part = Partition { cells: 0..n as u32, agents, ..Partition::default() };
        let mechanisms = mechanisms::defaults(&params);
        let schedule = schedule_of(&mechanisms).expect("default mechanisms form a valid schedule");
        World {
            config,
            grid,
            seed,
            tick,
            params,
            capacity,
            birth_counter,
            population,
            parts: vec![part],
            mechanisms: Arc::new(mechanisms),
            schedule,
        }
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
        Ok(())
    }

    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    pub fn partitions(&self) -> usize {
        self.parts.len()
    }

    /// Re-splits agents into `n` contiguous cell ranges (no-op if already `n`).
    pub fn set_partitions(&mut self, n: usize) {
        let n = n.max(1);
        if n == self.parts.len() {
            return;
        }
        let mut all = Agents::default();
        for p in &mut self.parts {
            all.append(&mut p.agents);
        }
        self.parts = self
            .grid
            .partition(n)
            .into_iter()
            .map(|cells| {
                let span = all.cell_range(cells.clone());
                let agents = Agents {
                    id: all.id[span.clone()].to_vec(),
                    cell: all.cell[span.clone()].to_vec(),
                    weight: all.weight[span].to_vec(),
                };
                Partition { cells, agents, ..Partition::default() }
            })
            .collect();
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
        self.set_partitions(parts);
        let ranges: Vec<Range<u32>> = self.parts.iter().map(|p| p.cells.clone()).collect();
        let nparts = ranges.len();

        // Sense + decide (parallel over partitions), running due mechanisms in order.
        {
            let sense = Sense {
                grid: &self.grid,
                capacity: &self.capacity,
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

        // Commit (parallel): canonical sort, merge small bands, recount population.
        let threshold = self.params.merge_threshold;
        let pops = split_by_ranges(&mut self.population, &ranges);
        let items: Vec<_> = self.parts.iter_mut().zip(pops).collect();
        par_for_each(items, |(part, pop)| {
            let cells = part.cells.clone();
            part.agents.sort_in_cells(cells.clone(), &mut part.scratch, &mut part.counts);
            part.agents.merge_small_into(threshold, &mut part.scratch, cells.start, pop);
            std::mem::swap(&mut part.agents, &mut part.scratch);
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
        let base = self.schedule.base_period();
        let steps = years * oikos_core::time::TICKS_PER_YEAR / base;
        let every = (summary_every * oikos_core::time::TICKS_PER_YEAR / base).max(1);
        let mut out = vec![self.summary()];
        for s in 1..=steps {
            self.step(parts);
            if s % every == 0 {
                out.push(self.summary());
            }
        }
        out
    }
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
