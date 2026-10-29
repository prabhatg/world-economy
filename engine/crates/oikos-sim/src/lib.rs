//! Oikos simulation runtime (M0).
//!
//! - [`agents`]: structure-of-arrays store of super-individual agents (each agent
//!   stands for `weight` people) with exact split/merge.
//! - [`world`]: world state and the bulk-synchronous phased step
//!   (sense → decide → exchange → commit → reduce) over spatial partitions.
//! - [`checkpoint`]: canonical binary snapshots; their BLAKE3 hash is the state hash
//!   volunteers' results are validated against.
//! - [`workunit`]: backend-neutral work units and results (docs/11 §11.2).
//! - [`inputs`]: the forcing packs a run reads, verified by id (docs/03 §3.8a).
//!
//! The M0 mechanism is a toy (logistic growth + band fission and migration); it
//! exists to exercise the engine, not to model history.

pub mod agents;
pub mod checkpoint;
pub mod environment;
pub mod inputs;
pub mod mechanisms;
pub mod output;
pub mod params;
pub mod schedule;
pub mod workunit;
pub mod world;

pub use params::Params;
pub use workunit::{WorkResult, WorkUnit, run_work_unit};
pub use world::{StepStats, Summary, World, WorldConfig};

/// Engine identity recorded in every result.
pub const ENGINE: &str = concat!("oikos ", env!("CARGO_PKG_VERSION"));
