//! I/O formats. Inputs: forcing packs of per-cell environmental fields
//! (docs/03-engine.md §3.8a). Outputs: Zarr v3 arrays for gridded fields and Parquet
//! tables for summaries and sampled agent panels (§3.8), written through a [`Sink`]
//! callback so hosts (CLI, browser, server) choose where bytes go.

pub mod pack;
pub mod table;
pub mod zarr;

/// Receives `(relative path, bytes)` for each output file.
pub type Sink<'a> = dyn FnMut(&str, &[u8]) -> Result<(), String> + 'a;

#[cfg(test)]
mod tests;
