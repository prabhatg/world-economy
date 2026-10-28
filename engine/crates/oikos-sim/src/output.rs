//! In-situ run outputs (docs/03-engine.md §3.8).
//!
//! Raw agent state is far too large to write every step, so a run records:
//! - `fields.zarr`: population per cell over time (Zarr v3), with the time coordinate
//!   (`tick`), each cell's H3 index (`cell_h3`) and carrying capacity;
//! - `summary.parquet`: one row per output time with totals and the flows since the
//!   previous row (births, deaths, fissions, migrants);
//! - `panel.parquet`: a deterministic sample of agents (by id hash) at every output time.
//!
//! Outputs only read the world, so recording never changes a run's state hash, and they
//! are byte-identical for any partition count.

use crate::world::{StepStats, World};
use oikos_core::EntityId;
use oikos_io::table::{self, Column};
use oikos_io::{Sink, zarr};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputSpec {
    /// Directory (relative to the host's output root) receiving all files.
    pub dir: String,
    #[serde(default = "default_every_years")]
    pub every_years: u64,
    /// Sample one agent in this many (by id hash) into the panel.
    #[serde(default = "default_panel_one_in")]
    pub panel_one_in: u64,
    #[serde(default = "default_time_chunk")]
    pub time_chunk: u64,
    #[serde(default = "default_cell_chunk")]
    pub cell_chunk: u64,
}

fn default_every_years() -> u64 {
    10
}
fn default_panel_one_in() -> u64 {
    64
}
fn default_time_chunk() -> u64 {
    16
}
fn default_cell_chunk() -> u64 {
    4096
}

impl OutputSpec {
    pub fn validate(&self) -> Result<(), String> {
        if self.every_years == 0 || self.panel_one_in == 0 || self.time_chunk == 0 || self.cell_chunk == 0 {
            return Err("outputs: every_years, panel_one_in, time_chunk and cell_chunk must be > 0".into());
        }
        Ok(())
    }
}

/// Deterministic panel membership: splitmix64 of the id, modulo `one_in`.
pub fn panel_sampled(id: EntityId, one_in: u64) -> bool {
    let mut z = id.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (z ^ (z >> 31)) % one_in == 0
}

#[derive(Default)]
struct Summary {
    tick: Vec<u64>,
    year: Vec<i64>,
    population: Vec<u64>,
    agents: Vec<u64>,
    occupied: Vec<u64>,
    births: Vec<u64>,
    deaths: Vec<u64>,
    fissions: Vec<u64>,
    migrants: Vec<u64>,
}

#[derive(Default)]
struct Panel {
    tick: Vec<u64>,
    id: Vec<u64>,
    cell: Vec<u64>,
    weight: Vec<u64>,
}

pub struct Recorder {
    spec: OutputSpec,
    population: zarr::TimeSeries,
    summary: Summary,
    panel: Panel,
    flows: StepStats,
    files: Vec<String>,
}

impl Recorder {
    /// Writes static arrays and records the initial state.
    pub fn start(world: &World, spec: OutputSpec, out: &mut Sink) -> Result<Recorder, String> {
        spec.validate()?;
        let cells = world.grid.len() as u64;
        let population = zarr::TimeSeries::new(
            "fields.zarr/population",
            cells,
            spec.time_chunk,
            spec.cell_chunk,
            &["time", "cell"],
            json!({ "unit": "people", "description": "Population per H3 cell at each output time" }),
        );
        let mut r = Recorder {
            spec,
            population,
            summary: Summary::default(),
            panel: Panel::default(),
            flows: StepStats::default(),
            files: Vec::new(),
        };
        let h3: Vec<u64> = (0..cells as u32).map(|c| world.grid.h3(c)).collect();
        let chunk = r.spec.cell_chunk;
        let attrs = json!({ "description": "H3 index of each cell" });
        r.write(out, |o| zarr::write_1d("fields.zarr/cell_h3", &h3, chunk, "cell", attrs, o))?;
        let attrs = json!({ "unit": "people", "description": "Carrying capacity per cell" });
        let cap = world.capacity.clone();
        r.write(out, |o| zarr::write_1d("fields.zarr/capacity", &cap, chunk, "cell", attrs, o))?;
        r.record(world, out)?;
        Ok(r)
    }

    /// Accumulates a step's flows; records a row when an output time is reached.
    pub fn observe(&mut self, world: &World, stats: StepStats, out: &mut Sink) -> Result<(), String> {
        self.flows.add(stats);
        let every = self.spec.every_years * oikos_core::time::TICKS_PER_YEAR;
        if world.tick.0 % every == 0 { self.record(world, out) } else { Ok(()) }
    }

    fn record(&mut self, world: &World, out: &mut Sink) -> Result<(), String> {
        let s = world.summary();
        let flows = std::mem::take(&mut self.flows);
        let sm = &mut self.summary;
        sm.tick.push(s.tick);
        sm.year.push(s.calendar_year);
        sm.population.push(s.population);
        sm.agents.push(s.agents);
        sm.occupied.push(s.occupied_cells);
        sm.births.push(flows.births);
        sm.deaths.push(flows.deaths);
        sm.fissions.push(flows.fissions);
        sm.migrants.push(flows.migrants);
        for a in world.agent_slices() {
            for i in 0..a.len() {
                if panel_sampled(a.id[i], self.spec.panel_one_in) {
                    self.panel.tick.push(s.tick);
                    self.panel.id.push(a.id[i].0);
                    self.panel.cell.push(a.cell[i] as u64);
                    self.panel.weight.push(a.weight[i]);
                }
            }
        }
        let dir = self.spec.dir.clone();
        let files = &mut self.files;
        let mut prefixed = |p: &str, b: &[u8]| {
            let path = format!("{dir}/{p}");
            out(&path, b)?;
            files.push(path);
            Ok(())
        };
        self.population.push(&world.population, &mut prefixed)
    }

    fn write(
        &mut self,
        out: &mut Sink,
        f: impl FnOnce(&mut Sink) -> Result<(), String>,
    ) -> Result<(), String> {
        let dir = self.spec.dir.clone();
        let files = &mut self.files;
        let mut prefixed = |p: &str, b: &[u8]| {
            let path = format!("{dir}/{p}");
            out(&path, b)?;
            files.push(path);
            Ok(())
        };
        f(&mut prefixed)
    }

    /// Writes remaining chunks, metadata and tables; returns every file written.
    pub fn finish(mut self, world: &World, out: &mut Sink) -> Result<Vec<String>, String> {
        let population = std::mem::replace(
            &mut self.population,
            zarr::TimeSeries::new("unused", 0, 1, 1, &["time", "cell"], json!({})),
        );
        self.write(out, |o| population.finish(o))?;
        let ticks = self.summary.tick.clone();
        let chunk = self.spec.time_chunk;
        let attrs =
            json!({ "unit": "days since 12000 BCE", "description": "Simulation tick of each output time" });
        self.write(out, |o| zarr::write_1d("fields.zarr/tick", &ticks, chunk, "time", attrs, o))?;
        let registry = crate::params::registry();
        let mut attrs = json!({
            "engine": crate::ENGINE,
            "seed": world.seed,
            "h3_resolution": world.grid.resolution(),
            "parameter_registry": format!("{} v{}", registry.name, registry.version),
            "params": world.params,
        });
        match &world.config {
            crate::WorldConfig::BaseCell(r) => attrs["h3_base_cell"] = r.base_cell.into(),
            crate::WorldConfig::Geometry(r) => {
                attrs["geometry_pack"] = r.geometry.clone().into();
                attrs["forcing_packs"] = world.inputs.ids().into();
            }
        }
        self.write(out, |o| zarr::write_group("fields.zarr", attrs, o))?;

        let sm = std::mem::take(&mut self.summary);
        let summary = table::write(&[
            Column::U64("tick", sm.tick),
            Column::I64("calendar_year", sm.year),
            Column::U64("population", sm.population),
            Column::U64("agents", sm.agents),
            Column::U64("occupied_cells", sm.occupied),
            Column::U64("births", sm.births),
            Column::U64("deaths", sm.deaths),
            Column::U64("fissions", sm.fissions),
            Column::U64("migrants", sm.migrants),
        ])?;
        self.write(out, |o| o("summary.parquet", &summary))?;
        let p = std::mem::take(&mut self.panel);
        let panel = table::write(&[
            Column::U64("tick", p.tick),
            Column::U64("id", p.id),
            Column::U64("cell", p.cell),
            Column::U64("weight", p.weight),
        ])?;
        self.write(out, |o| o("panel.parquet", &panel))?;
        let mut files = self.files;
        files.sort();
        files.dedup();
        Ok(files)
    }
}
