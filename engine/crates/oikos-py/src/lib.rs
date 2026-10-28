//! Python bindings for the Oikos engine (module `oikos`, built with maturin).
//!
//! Calibration and analysis code (docs/05) creates, runs, forks and reads worlds
//! in-process instead of shelling out to the `oikos` CLI. The bindings are thin wrappers
//! over `oikos-sim`: no simulation logic lives here, so a world stepped from Python has
//! the same state hash as the same run from the CLI, a work unit or a volunteer.
//!
//! - Structured values (summaries, stats, params, registry, work-unit results) cross
//!   the boundary as the engine's own JSON, so Python sees exactly the schema the CLI
//!   prints and work units use.
//! - Per-cell and per-agent arrays are returned as NumPy arrays the caller owns.
//! - Engine errors (`Err(String)`) become `ValueError` with the same message.
//! - Long calls (`run_years`, `run_work_unit`) release the GIL, so threads can run
//!   independent worlds in parallel.

use oikos_params::Value;
use oikos_sim::inputs::Inputs;
use oikos_sim::{World, WorldConfig, checkpoint, params::registry};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyByteArray, PyBytes, PyDict, PyFloat, PyInt};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn value_err(e: impl Into<String>) -> PyErr {
    PyValueError::new_err(e.into())
}

/// Any serializable engine value as the Python object `json.loads` makes of it.
fn to_py<'py>(py: Python<'py>, v: &impl Serialize) -> PyResult<Bound<'py, PyAny>> {
    let s = serde_json::to_string(v).map_err(|e| value_err(e.to_string()))?;
    py.import("json")?.call_method1("loads", (s,))
}

/// Little-endian integers as a writable NumPy array (backed by a bytearray it owns).
fn array<'py>(py: Python<'py>, dtype: &str, bytes: Vec<u8>) -> PyResult<Bound<'py, PyAny>> {
    let buf = PyByteArray::new(py, &bytes);
    py.import("numpy")?.call_method1("frombuffer", (buf, dtype))
}

fn u64_array<'py>(py: Python<'py>, values: impl Iterator<Item = u64>) -> PyResult<Bound<'py, PyAny>> {
    array(py, "<u8", values.flat_map(u64::to_le_bytes).collect())
}

/// One parameter value from Python: ints stay integers (counts never pass through
/// floats), floats are reals; bools and negative ints are rejected.
fn param_value(key: &str, v: &Bound<'_, PyAny>) -> PyResult<Value> {
    if v.is_instance_of::<PyBool>() {
        return Err(value_err(format!("parameter {key}: must be a number, got {v}")));
    }
    if v.is_instance_of::<PyFloat>() {
        return Ok(Value::Real(v.extract()?));
    }
    if let Ok(i) = v.extract::<u64>() {
        return Ok(Value::Int(i));
    }
    if v.is_instance_of::<PyInt>() {
        return Err(value_err(format!("parameter {key}: integers must be in [0, 2^64), got {v}")));
    }
    v.extract::<f64>()
        .map(Value::Real)
        .map_err(|_| value_err(format!("parameter {key}: must be a number, got {}", v.get_type())))
}

fn overrides(params: &Bound<'_, PyDict>) -> PyResult<BTreeMap<String, Value>> {
    params
        .iter()
        .map(|(k, v)| {
            let key: String = k.extract()?;
            let value = param_value(&key, &v)?;
            Ok((key, value))
        })
        .collect()
}

fn params_from(params: Option<&Bound<'_, PyDict>>) -> PyResult<oikos_sim::Params> {
    match params {
        None => Ok(oikos_sim::Params::default()),
        Some(d) => oikos_sim::Params::from_registry(registry(), &overrides(d)?).map_err(value_err),
    }
}

/// A simulated world. Same seed and parameters give the same history, bit for bit,
/// for any partition count and on every host.
#[pyclass(module = "oikos", name = "World")]
struct PyWorld(World);

#[pymethods]
impl PyWorld {
    /// A fresh world on H3 `base_cell` at `resolution`, with environment and initial
    /// bands drawn from `seed`. `params` overrides registry defaults (range-checked).
    #[new]
    #[pyo3(signature = (base_cell = 22, resolution = 4, *, seed, params = None))]
    fn new(base_cell: u8, resolution: u8, seed: u64, params: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let config = WorldConfig::base_cell(base_cell, resolution);
        World::new(config, seed, params_from(params)?).map(PyWorld).map_err(value_err)
    }

    /// A fresh world on the region of a geometry pack, with its forcing packs (paths
    /// to `.oikospack` files: exactly one geometry pack, at most one of each kind).
    #[staticmethod]
    #[pyo3(signature = (packs, *, seed, params = None))]
    fn from_packs(packs: Vec<PathBuf>, seed: u64, params: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let inputs = load_packs(&packs)?;
        let geometry = inputs.geometry().map(|g| g.id().to_string()).unwrap_or_default();
        let config = WorldConfig::geometry(&geometry);
        World::with_inputs(config, seed, params_from(params)?, inputs).map(PyWorld).map_err(value_err)
    }

    /// Restores a world from `checkpoint()` bytes, with the forcing packs it was made
    /// with (paths), if any.
    #[staticmethod]
    #[pyo3(signature = (data, packs = None))]
    fn from_checkpoint(data: &[u8], packs: Option<Vec<PathBuf>>) -> PyResult<Self> {
        let inputs = load_packs(&packs.unwrap_or_default())?;
        checkpoint::decode_with(data, inputs).map(PyWorld).map_err(value_err)
    }

    /// Canonical binary snapshot; `World.from_checkpoint` restores it exactly.
    fn checkpoint<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &checkpoint::encode(&self.0))
    }

    /// A new branch of history from this state: same agents and environment, new run
    /// seed and optionally new parameters (as a work unit's checkpoint `fork`).
    #[pyo3(signature = (seed, params = None))]
    fn fork(&self, seed: u64, params: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut w = checkpoint::decode_with(&checkpoint::encode(&self.0), self.0.inputs.clone())
            .map_err(value_err)?;
        w.seed = seed;
        if params.is_some() {
            w.set_params(params_from(params)?).map_err(value_err)?;
        }
        Ok(PyWorld(w))
    }

    /// Advances one base period; returns its exact births, deaths, fissions, migrants.
    #[pyo3(signature = (parts = 1))]
    fn step<'py>(&mut self, py: Python<'py>, parts: usize) -> PyResult<Bound<'py, PyAny>> {
        let stats = self.0.step(parts);
        to_py(py, &stats)
    }

    /// Runs `years` years on `parts` partitions; returns the summaries (the starting
    /// state, then one every `summary_every` years), as in a work-unit result.
    #[pyo3(signature = (years, parts = 1, summary_every = 100))]
    fn run_years<'py>(
        &mut self,
        py: Python<'py>,
        years: u64,
        parts: usize,
        summary_every: u64,
    ) -> PyResult<Bound<'py, PyAny>> {
        if summary_every == 0 {
            return Err(value_err("summary_every must be at least 1 year"));
        }
        let world = &mut self.0;
        let summaries = py.detach(|| world.run_years(years, parts, summary_every));
        to_py(py, &summaries)
    }

    /// Population totals now: tick, calendar_year, population, agents, occupied_cells.
    fn summary<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        to_py(py, &self.0.summary())
    }

    /// Hex BLAKE3 hash of the canonical state; equal hashes mean identical worlds.
    fn state_hash(&self) -> String {
        checkpoint::state_hash(&self.0)
    }

    /// People per cell (uint64, one entry per grid cell).
    fn population<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        u64_array(py, self.0.population.iter().copied())
    }

    /// Agents in canonical (cell, id) order: `id` (uint64), `cell` (uint32 grid index)
    /// and `weight` (uint64, people the agent stands for).
    fn agents<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let slices = || self.0.agent_slices();
        let out = PyDict::new(py);
        out.set_item("id", u64_array(py, slices().flat_map(|a| a.id.iter().map(|id| id.0)))?)?;
        let cells = slices().flat_map(|a| a.cell.iter().flat_map(|c| c.to_le_bytes())).collect();
        out.set_item("cell", array(py, "<u4", cells)?)?;
        out.set_item("weight", u64_array(py, slices().flat_map(|a| a.weight.iter().copied()))?)?;
        Ok(out)
    }

    /// Days since the simulation epoch.
    #[getter]
    fn tick(&self) -> u64 {
        self.0.tick.0
    }

    #[getter]
    fn seed(&self) -> u64 {
        self.0.seed
    }

    /// Every parameter value of this world (registry defaults plus overrides).
    #[getter]
    fn params<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        to_py(py, &self.0.params)
    }

    /// H3 base cell of a base-cell world; `None` for a geometry-pack region.
    #[getter]
    fn base_cell(&self) -> Option<u8> {
        match &self.0.config {
            WorldConfig::BaseCell(r) => Some(r.base_cell),
            WorldConfig::Geometry(_) => None,
        }
    }

    /// Id of the geometry pack defining the region; `None` for a base-cell world.
    #[getter]
    fn geometry(&self) -> Option<String> {
        match &self.0.config {
            WorldConfig::BaseCell(_) => None,
            WorldConfig::Geometry(r) => Some(r.geometry.clone()),
        }
    }

    /// Ids of the forcing packs this world reads (sorted).
    #[getter]
    fn inputs(&self) -> Vec<String> {
        self.0.inputs.ids().into_iter().map(String::from).collect()
    }

    #[getter]
    fn resolution(&self) -> u8 {
        self.0.grid.resolution()
    }

    /// Number of grid cells.
    #[getter]
    fn cells(&self) -> usize {
        self.0.grid.len()
    }

    fn __repr__(&self) -> String {
        let w = &self.0;
        let region = match &w.config {
            WorldConfig::BaseCell(r) => format!("base_cell={}", r.base_cell),
            WorldConfig::Geometry(r) => format!("geometry={}", &r.geometry[..12]),
        };
        format!(
            "World({region}, resolution={}, cells={}, seed={}, tick={})",
            w.grid.resolution(),
            w.grid.len(),
            w.seed,
            w.tick.0
        )
    }
}

/// Runs a work unit (JSON text) and returns its result, exactly as `oikos run` does.
/// Paths in the work unit are read relative to `read_dir` (checkpoints) and written
/// relative to `write_dir` (checkpoint_out, outputs).
#[pyfunction]
#[pyo3(signature = (work_unit, read_dir = PathBuf::from("."), write_dir = PathBuf::from(".")))]
fn run_work_unit<'py>(
    py: Python<'py>,
    work_unit: &str,
    read_dir: PathBuf,
    write_dir: PathBuf,
) -> PyResult<Bound<'py, PyAny>> {
    let wu: oikos_sim::WorkUnit =
        serde_json::from_str(work_unit).map_err(|e| value_err(format!("work unit: {e}")))?;
    let result = py
        .detach(|| {
            oikos_sim::run_work_unit(&wu, |p| read(&read_dir.join(p)), |p, b| write(&write_dir.join(p), b))
        })
        .map_err(value_err)?;
    to_py(py, &result)
}

/// The parameter registry (units, ranges, priors, sources), as `oikos params` prints.
#[pyfunction(name = "registry")]
fn registry_py(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    py.import("json")?.call_method1("loads", (registry().to_json(),))
}

/// Maps unit-interval values `{key: u}` (u in [0, 1]) onto each parameter's prior and
/// returns the overrides `{key: value}`, ready for `World(params=...)`. Uniform and
/// log-uniform priors are supported; integer parameters are rounded.
#[pyfunction]
fn sample_params<'py>(py: Python<'py>, u: &Bound<'py, PyDict>) -> PyResult<Bound<'py, PyAny>> {
    let mut unit = BTreeMap::new();
    for (k, v) in u.iter() {
        let key: String = k.extract()?;
        let x = match param_value(&key, &v)? {
            Value::Int(i) => i as f64,
            Value::Real(x) => x,
        };
        unit.insert(key, x);
    }
    let values = registry().sample(&unit).map_err(value_err)?;
    to_py(py, &registry().values_json(&values))
}

/// Forcing packs from files (each pack's id is the hash of its bytes).
fn load_packs(paths: &[PathBuf]) -> PyResult<Inputs> {
    let packs = paths
        .iter()
        .map(|p| oikos_io::pack::Pack::parse(read(p)?).map_err(|e| format!("{}: {e}", p.display())))
        .collect::<Result<Vec<_>, String>>()
        .map_err(value_err)?;
    Inputs::from_packs(packs).map_err(value_err)
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

#[pymodule]
#[pyo3(name = "oikos")]
fn oikos_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWorld>()?;
    m.add_function(wrap_pyfunction!(run_work_unit, m)?)?;
    m.add_function(wrap_pyfunction!(registry_py, m)?)?;
    m.add_function(wrap_pyfunction!(sample_params, m)?)?;
    m.add("ENGINE", oikos_sim::ENGINE)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
