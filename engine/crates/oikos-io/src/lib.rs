//! Output formats: Zarr v3 arrays for gridded fields and Parquet tables for summaries
//! and sampled agent panels (docs/03-engine.md §3.8). Everything is written through a
//! [`Sink`] callback so hosts (CLI, browser, server) choose where bytes go.

pub mod table;
pub mod zarr;

/// Receives `(relative path, bytes)` for each output file.
pub type Sink<'a> = dyn FnMut(&str, &[u8]) -> Result<(), String> + 'a;

#[cfg(test)]
mod tests;
