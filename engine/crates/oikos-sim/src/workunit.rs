//! Backend-neutral work units (docs/11 §11.2).
//!
//! The same JSON work unit runs on a volunteer browser, desktop, phone, cloud spot VM
//! or HPC node. Results contain only integers and hashes so they compare exactly.

use crate::checkpoint;
use crate::params::Params;
use crate::world::{Summary, World, WorldConfig};
use serde::{Deserialize, Serialize};

pub const FORMAT: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Start {
    /// A fresh world from `world`, `seed` and `params`.
    Initial,
    /// Continue from a checkpoint (`seed`/`params` in the work unit are ignored unless
    /// `fork` is set).
    Checkpoint {
        path: String,
        /// Fork: replace the run seed (and params, if given) — a new branch of history.
        #[serde(default)]
        fork: bool,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkUnit {
    pub format: u32,
    #[serde(default)]
    pub id: Option<String>,
    pub world: WorldConfig,
    pub seed: u64,
    #[serde(default)]
    pub params: Option<Params>,
    pub start: Start,
    pub years: u64,
    #[serde(default = "default_partitions")]
    pub partitions: usize,
    #[serde(default = "default_summary_every")]
    pub summary_every_years: u64,
    #[serde(default)]
    pub checkpoint_out: Option<String>,
}

fn default_partitions() -> usize {
    1
}

fn default_summary_every() -> u64 {
    100
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkResult {
    pub format: u32,
    pub id: Option<String>,
    pub engine: String,
    pub final_tick: u64,
    /// BLAKE3 of the canonical final-state encoding; volunteers' results must match.
    pub state_hash: String,
    pub summaries: Vec<Summary>,
    pub checkpoint: Option<String>,
}

/// Runs a work unit. `read`/`write` abstract storage so hosts (browser, CLI, server)
/// can supply their own I/O.
pub fn run_work_unit(
    wu: &WorkUnit,
    read: impl Fn(&str) -> Result<Vec<u8>, String>,
    write: impl Fn(&str, &[u8]) -> Result<(), String>,
) -> Result<WorkResult, String> {
    if wu.format != FORMAT {
        return Err(format!("unsupported work unit format {}", wu.format));
    }
    let mut world = match &wu.start {
        Start::Initial => World::new(wu.world, wu.seed, wu.params.clone().unwrap_or_default())?,
        Start::Checkpoint { path, fork } => {
            let mut w = checkpoint::decode(&read(path)?)?;
            if w.config != wu.world {
                return Err("checkpoint world does not match work unit".into());
            }
            if *fork {
                w.seed = wu.seed;
                if let Some(p) = &wu.params {
                    p.validate()?;
                    w.params = p.clone();
                }
            }
            w
        }
    };
    let summaries = world.run_years(wu.years, wu.partitions.max(1), wu.summary_every_years.max(1));
    if let Some(path) = &wu.checkpoint_out {
        write(path, &checkpoint::encode(&world))?;
    }
    Ok(WorkResult {
        format: FORMAT,
        id: wu.id.clone(),
        engine: crate::ENGINE.to_string(),
        final_tick: world.tick.0,
        state_hash: checkpoint::state_hash(&world),
        summaries,
        checkpoint: wu.checkpoint_out.clone(),
    })
}
