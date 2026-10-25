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
use crate::params::Params;
use oikos_core::prob::stochastic_round;
use oikos_core::{EntityId, Philox4x32, Probability, Stream, Tick};
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

/// A contiguous range of cells and the agents living in them.
#[derive(Default)]
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
        World { config, grid, seed, tick, params, capacity, birth_counter, population, parts: vec![part] }
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

    /// Advances one mechanism period using `parts` spatial partitions.
    pub fn step(&mut self, parts: usize) -> StepStats {
        self.set_partitions(parts);
        let ranges: Vec<Range<u32>> = self.parts.iter().map(|p| p.cells.clone()).collect();
        let nparts = ranges.len();

        // Sense + decide (parallel over partitions).
        {
            let ctx = StepCtx {
                grid: &self.grid,
                params: &self.params,
                capacity: &self.capacity,
                population: &self.population,
                ranges: &ranges,
                tick: self.tick,
                growth: Philox4x32::new(self.seed, Stream::Growth),
                fission: Philox4x32::new(self.seed, Stream::Fission),
                migration: Philox4x32::new(self.seed, Stream::Migration),
            };
            let counters = split_by_ranges(&mut self.birth_counter, &ranges);
            let items: Vec<_> = self.parts.iter_mut().zip(counters).collect();
            par_for_each(items, |(part, counters)| decide(&ctx, part, counters));
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
        self.tick = self.tick.advance(self.params.period_ticks);
        stats
    }

    /// Runs `years` simulated years, recording a summary every `summary_every` years.
    pub fn run_years(&mut self, years: u64, parts: usize, summary_every: u64) -> Vec<Summary> {
        let steps = years * oikos_core::time::TICKS_PER_YEAR / self.params.period_ticks;
        let every = (summary_every * oikos_core::time::TICKS_PER_YEAR / self.params.period_ticks).max(1);
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

struct StepCtx<'a> {
    grid: &'a Grid,
    params: &'a Params,
    capacity: &'a [u64],
    population: &'a [u64],
    ranges: &'a [Range<u32>],
    tick: Tick,
    growth: Philox4x32,
    fission: Philox4x32,
    migration: Philox4x32,
}

/// Decide phase for one partition, in place. `counters` are the birth counters of the
/// partition's cells. Daughters that stay are appended to the partition's agents;
/// daughters that leave go to the outbox of the destination partition.
fn decide(ctx: &StepCtx, part: &mut Partition, counters: &mut [u64]) {
    let p = ctx.params;
    let t = ctx.tick.0;
    let fission_p = Probability::from_f64(p.fission_probability);
    let explore_p = Probability::from_f64(p.exploration_probability);
    let base = part.cells.start;
    let mut stats = StepStats::default();
    part.outbox.resize_with(ctx.ranges.len(), Agents::default);
    part.outbox.iter_mut().for_each(Agents::clear);
    let agents = &mut part.agents;

    let n = agents.len();
    for i in 0..n {
        let id = agents.id[i];
        let cell = agents.cell[i];
        let mut w = agents.weight[i];

        // Logistic growth: dw = r w (1 - N/K), stochastically rounded.
        let pop = ctx.population[cell as usize] as f64;
        let k = ctx.capacity[cell as usize] as f64;
        let dw = oikos_math::finite(p.growth_rate * w as f64 * (1.0 - pop / k));
        let draw = ctx.growth.u32(id.0, t, 0, 0);
        if dw >= 0.0 {
            let b = stochastic_round(dw, draw);
            w += b;
            stats.births += b;
        } else {
            let d = stochastic_round(-dw, draw).min(w);
            w -= d;
            stats.deaths += d;
        }

        // Fission: large bands split; the daughter moves to a less crowded neighbor.
        if w >= p.fission_threshold && fission_p.hit(ctx.fission.u32(id.0, t, 0, 0)) {
            let take = w / 2;
            let half = Agents::split_weight(&mut w, take);
            let child = next_id_local(counters, base, cell);
            stats.fissions += 1;
            let dest = choose_destination(ctx, id, cell, explore_p);
            if dest == cell {
                agents.push(child, cell, half);
            } else {
                let owner = ctx.ranges.partition_point(|r| r.end <= dest);
                part.outbox[owner].push(child, dest, half);
                stats.migrants += half;
            }
        }
        agents.weight[i] = w;
    }
    part.stats = stats;
}

fn next_id_local(counters: &mut [u64], base: u32, cell: u32) -> EntityId {
    let c = &mut counters[(cell - base) as usize];
    let id = EntityId::new(cell, *c);
    *c += 1;
    id
}

/// Least crowded neighbor (by N/K), or a random one with the exploration probability.
/// Returns `cell` itself if no neighbor is less crowded.
fn choose_destination(ctx: &StepCtx, id: EntityId, cell: u32, explore_p: Probability) -> u32 {
    let ns = ctx.grid.neighbors(cell);
    if ns.is_empty() {
        return cell;
    }
    let draws = ctx.migration.block(id.0, ctx.tick.0, 0);
    if explore_p.hit(draws[0]) {
        return ns[(((draws[1] as u64) * (ns.len() as u64)) >> 32) as usize];
    }
    // Compare N_a / K_a < N_b / K_b exactly as N_a K_b < N_b K_a.
    let crowd = |c: u32| (ctx.population[c as usize] as u128, ctx.capacity[c as usize] as u128);
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
