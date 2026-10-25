//! World state and the phased step.
//!
//! Each step is bulk-synchronous:
//! 1. **sense**   — read the previous step's cell populations (a read-only snapshot,
//!    i.e. the halo in a distributed run);
//! 2. **decide**  — each partition updates its agents and emits migrants;
//! 3. **exchange** — migrants are routed to the partition owning their new cell;
//! 4. **commit**  — each partition sorts, merges small bands, recomputes cell totals;
//! 5. **reduce**  — integer summaries.
//!
//! Every random draw is keyed by (seed, stream, entity, tick) and every reduction is an
//! integer sum, so the result is identical for any partition count.

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
    pub const SOUTHWEST_ASIA: WorldConfig = WorldConfig {
        base_cell: 22,
        resolution: 4,
    };

    pub fn build_grid(&self) -> Result<Grid, String> {
        Grid::from_base_cell(self.base_cell, self.resolution).map_err(|e| e.to_string())
    }
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
    /// Agents in canonical (cell, id) order.
    pub agents: Agents,
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

        let mut world = World {
            config,
            grid,
            seed,
            tick: Tick::EPOCH,
            params,
            capacity,
            birth_counter: vec![0; n],
            population: vec![0; n],
            agents: Agents::default(),
        };

        // Initial bands cluster around one seed-chosen cell and its neighbors.
        let init = Philox4x32::new(seed, Stream::Initialization);
        let center = init.below(0, 0, 0, 0, n as u32);
        let mut home: Vec<u32> = vec![center];
        home.extend_from_slice(world.grid.neighbors(center));
        for b in 0..world.params.initial_bands {
            let cell = home[init.below(1, 0, b, 0, home.len() as u32) as usize];
            let id = world.new_id(cell);
            world.agents.push(id, cell, world.params.initial_weight);
        }
        world.agents.sort_canonical();
        world.recount();
        Ok(world)
    }

    fn new_id(&mut self, cell: u32) -> EntityId {
        let c = &mut self.birth_counter[cell as usize];
        let id = EntityId::new(cell, *c);
        *c += 1;
        id
    }

    fn recount(&mut self) {
        self.population.iter_mut().for_each(|p| *p = 0);
        for (&c, &w) in self.agents.cell.iter().zip(&self.agents.weight) {
            self.population[c as usize] += w;
        }
    }

    pub fn total_population(&self) -> u64 {
        self.population.iter().sum()
    }

    pub fn summary(&self) -> Summary {
        Summary {
            tick: self.tick.0,
            calendar_year: self.tick.calendar_year(),
            population: self.total_population(),
            agents: self.agents.len() as u64,
            occupied_cells: self.population.iter().filter(|&&p| p > 0).count() as u64,
        }
    }

    /// Advances one mechanism period using `parts` spatial partitions.
    pub fn step(&mut self, parts: usize) -> StepStats {
        let ranges = self.grid.partition(parts.max(1));
        let ctx = StepCtx {
            grid: &self.grid,
            params: &self.params,
            capacity: &self.capacity,
            population: &self.population,
            tick: self.tick,
            growth: Philox4x32::new(self.seed, Stream::Growth),
            fission: Philox4x32::new(self.seed, Stream::Fission),
            migration: Philox4x32::new(self.seed, Stream::Migration),
        };

        // Split owned state into per-partition slices (sense + decide).
        let agents = std::mem::take(&mut self.agents);
        let mut inputs = Vec::with_capacity(ranges.len());
        let mut counters: &mut [u64] = &mut self.birth_counter;
        for r in &ranges {
            let (mine, rest) = std::mem::take(&mut counters).split_at_mut((r.end - r.start) as usize);
            counters = rest;
            let span = agents.cell_range(r.clone());
            let slice = Agents {
                id: agents.id[span.clone()].to_vec(),
                cell: agents.cell[span.clone()].to_vec(),
                weight: agents.weight[span].to_vec(),
            };
            inputs.push((r.clone(), slice, mine));
        }
        let outputs = run_parallel(inputs, |(range, slice, counters)| {
            decide(&ctx, range, slice, counters)
        });

        // Exchange: route migrants to the partition owning their destination cell.
        let mut stats = StepStats::default();
        let mut staying: Vec<Agents> = Vec::with_capacity(ranges.len());
        let mut inbox: Vec<Agents> = vec![Agents::default(); ranges.len()];
        for out in outputs {
            stats.add(out.stats);
            staying.push(out.staying);
            let m = out.migrants;
            for i in 0..m.len() {
                let owner = ranges.partition_point(|r| r.end <= m.cell[i]);
                inbox[owner].push(m.id[i], m.cell[i], m.weight[i]);
            }
        }

        // Commit per partition, then concatenate (partitions are in cell order).
        let merge_threshold = self.params.merge_threshold;
        let committed = run_parallel(
            staying.into_iter().zip(inbox).collect(),
            |(mut mine, mut incoming)| {
                mine.append(&mut incoming);
                mine.sort_canonical();
                mine.merge_small(merge_threshold);
                mine
            },
        );
        let mut all = Agents::default();
        for mut part in committed {
            all.append(&mut part);
        }
        self.agents = all;
        self.recount();
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

struct StepCtx<'a> {
    grid: &'a Grid,
    params: &'a Params,
    capacity: &'a [u64],
    population: &'a [u64],
    tick: Tick,
    growth: Philox4x32,
    fission: Philox4x32,
    migration: Philox4x32,
}

struct DecideOut {
    staying: Agents,
    migrants: Agents,
    stats: StepStats,
}

/// Decide phase for one partition. `counters` are the birth counters of `range`.
fn decide(ctx: &StepCtx, range: Range<u32>, agents: Agents, counters: &mut [u64]) -> DecideOut {
    let p = ctx.params;
    let t = ctx.tick.0;
    let fission_p = Probability::from_f64(p.fission_probability);
    let explore_p = Probability::from_f64(p.exploration_probability);
    let mut out = DecideOut {
        staying: Agents::default(),
        migrants: Agents::default(),
        stats: StepStats::default(),
    };

    for i in 0..agents.len() {
        let id = agents.id[i];
        let cell = agents.cell[i];
        let mut w = agents.weight[i];

        // Logistic growth: dw = r w (1 - N/K), stochastically rounded.
        let n = ctx.population[cell as usize] as f64;
        let k = ctx.capacity[cell as usize] as f64;
        let dw = oikos_math::finite(p.growth_rate * w as f64 * (1.0 - n / k));
        let draw = ctx.growth.u32(id.0, t, 0, 0);
        if dw >= 0.0 {
            let b = stochastic_round(dw, draw);
            w += b;
            out.stats.births += b;
        } else {
            let d = stochastic_round(-dw, draw).min(w);
            w -= d;
            out.stats.deaths += d;
        }

        // Fission: large bands split; the daughter moves to a less crowded neighbor.
        if w >= p.fission_threshold && fission_p.hit(ctx.fission.u32(id.0, t, 0, 0)) {
            let take = w / 2;
            let half = Agents::split_weight(&mut w, take);
            let counter = &mut counters[(cell - range.start) as usize];
            let child = EntityId::new(cell, *counter);
            *counter += 1;
            out.stats.fissions += 1;
            let dest = choose_destination(ctx, id, cell, explore_p);
            if dest == cell {
                out.staying.push(child, cell, half);
            } else {
                out.migrants.push(child, dest, half);
                out.stats.migrants += half;
            }
        }
        out.staying.push(id, cell, w);
    }
    out
}

/// Least crowded neighbor (by N/K), or a random one with the exploration probability.
/// Returns `cell` itself if no neighbor is less crowded.
fn choose_destination(ctx: &StepCtx, id: EntityId, cell: u32, explore_p: Probability) -> u32 {
    let ns = ctx.grid.neighbors(cell);
    if ns.is_empty() {
        return cell;
    }
    let t = ctx.tick.0;
    if explore_p.hit(ctx.migration.u32(id.0, t, 0, 0)) {
        return ns[ctx.migration.below(id.0, t, 0, 1, ns.len() as u32) as usize];
    }
    // Compare N_a / K_a < N_b / K_b exactly as N_a K_b < N_b K_a.
    let crowd = |c: u32| {
        (
            ctx.population[c as usize] as u128,
            ctx.capacity[c as usize] as u128,
        )
    };
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

/// Runs `f` over `items`, in parallel on native targets and sequentially on wasm.
/// Output order always equals input order.
fn run_parallel<I: Send, O: Send>(items: Vec<I>, f: impl Fn(I) -> O + Sync) -> Vec<O> {
    if cfg!(target_family = "wasm") || items.len() <= 1 {
        return items.into_iter().map(f).collect();
    }
    let f = &f;
    std::thread::scope(|s| {
        let handles: Vec<_> = items.into_iter().map(|it| s.spawn(move || f(it))).collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("partition worker panicked"))
            .collect()
    })
}
