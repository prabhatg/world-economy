//! Core types shared by every Oikos crate.
//!
//! Everything here is integer-based and platform-independent so that runs are
//! bit-reproducible across x86_64, aarch64 and wasm32 (see docs/03-engine.md §3.5).

#![no_std]

pub mod id;
pub mod prob;
pub mod rng;
pub mod time;

pub use id::EntityId;
pub use prob::Probability;
pub use rng::{Philox4x32, Stream};
pub use time::Tick;
