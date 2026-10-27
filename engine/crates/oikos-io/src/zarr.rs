//! Minimal Zarr v3 writer for unsigned 64-bit arrays.
//!
//! Only what the engine needs: groups, 1-D static arrays, and 2-D (time × cell) arrays
//! that grow along time as a run proceeds. Chunks use the `bytes` (little-endian) and
//! `gzip` codecs; edge chunks are stored at full chunk shape padded with the fill value,
//! as the regular chunk grid requires. Output bytes are deterministic. Readable by
//! zarr-python 3 and xarray.

use crate::Sink;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::{Value, json};
use std::io::Write;

const GZIP_LEVEL: u32 = 5;

/// `zarr.json` for a uint64 array.
pub fn array_metadata(shape: &[u64], chunks: &[u64], dims: &[&str], attributes: Value) -> String {
    let meta = json!({
        "zarr_format": 3,
        "node_type": "array",
        "shape": shape,
        "data_type": "uint64",
        "chunk_grid": { "name": "regular", "configuration": { "chunk_shape": chunks } },
        "chunk_key_encoding": { "name": "default", "configuration": { "separator": "/" } },
        "fill_value": 0,
        "codecs": [
            { "name": "bytes", "configuration": { "endian": "little" } },
            { "name": "gzip", "configuration": { "level": GZIP_LEVEL } }
        ],
        "attributes": attributes,
        "dimension_names": dims,
    });
    serde_json::to_string_pretty(&meta).expect("metadata serializes")
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() { name.to_string() } else { format!("{prefix}/{name}") }
}

/// Writes a group's `zarr.json` at `path` ("" for the store root).
pub fn write_group(path: &str, attributes: Value, out: &mut Sink) -> Result<(), String> {
    let meta = json!({ "zarr_format": 3, "node_type": "group", "attributes": attributes });
    out(&join(path, "zarr.json"), serde_json::to_string_pretty(&meta).expect("serializes").as_bytes())
}

fn encode(values: &[u64]) -> Vec<u8> {
    let mut gz = GzEncoder::new(Vec::with_capacity(values.len() * 2), Compression::new(GZIP_LEVEL));
    for v in values {
        gz.write_all(&v.to_le_bytes()).expect("in-memory write");
    }
    gz.finish().expect("in-memory gzip")
}

/// Writes a 1-D array `name` with chunks of `chunk` elements.
pub fn write_1d(
    name: &str,
    data: &[u64],
    chunk: u64,
    dim: &str,
    attributes: Value,
    out: &mut Sink,
) -> Result<(), String> {
    assert!(chunk > 0);
    for (i, part) in data.chunks(chunk as usize).enumerate() {
        let mut padded = part.to_vec();
        padded.resize(chunk as usize, 0);
        out(&format!("{name}/c/{i}"), &encode(&padded))?;
    }
    let meta = array_metadata(&[data.len() as u64], &[chunk], &[dim], attributes);
    out(&join(name, "zarr.json"), meta.as_bytes())
}

/// A 2-D array of shape (time, cells) written row by row; chunks are emitted as soon
/// as a band of `time_chunk` rows is complete, so memory stays bounded.
pub struct TimeSeries {
    name: String,
    cells: u64,
    time_chunk: u64,
    cell_chunk: u64,
    dims: [String; 2],
    attributes: Value,
    band: Vec<u64>,
    band_rows: u64,
    rows: u64,
}

impl TimeSeries {
    pub fn new(
        name: &str,
        cells: u64,
        time_chunk: u64,
        cell_chunk: u64,
        dims: &[&str; 2],
        attributes: Value,
    ) -> TimeSeries {
        assert!(time_chunk > 0 && cell_chunk > 0);
        TimeSeries {
            name: name.to_string(),
            cells,
            time_chunk,
            cell_chunk,
            dims: [dims[0].to_string(), dims[1].to_string()],
            attributes,
            band: Vec::new(),
            band_rows: 0,
            rows: 0,
        }
    }

    pub fn push(&mut self, row: &[u64], out: &mut Sink) -> Result<(), String> {
        if row.len() as u64 != self.cells {
            return Err(format!("{}: row has {} values, expected {}", self.name, row.len(), self.cells));
        }
        self.band.extend_from_slice(row);
        self.band_rows += 1;
        self.rows += 1;
        if self.band_rows == self.time_chunk { self.flush(out) } else { Ok(()) }
    }

    fn flush(&mut self, out: &mut Sink) -> Result<(), String> {
        if self.band_rows == 0 {
            return Ok(());
        }
        let ti = (self.rows - self.band_rows) / self.time_chunk;
        let (tc, cc) = (self.time_chunk as usize, self.cell_chunk as usize);
        let n_cell_chunks = self.cells.div_ceil(self.cell_chunk);
        let mut chunk = vec![0u64; tc * cc];
        for ci in 0..n_cell_chunks {
            chunk.iter_mut().for_each(|v| *v = 0);
            let c0 = (ci * self.cell_chunk) as usize;
            let width = cc.min(self.cells as usize - c0);
            for r in 0..self.band_rows as usize {
                let src = &self.band[r * self.cells as usize + c0..][..width];
                chunk[r * cc..r * cc + width].copy_from_slice(src);
            }
            out(&format!("{}/c/{ti}/{ci}", self.name), &encode(&chunk))?;
        }
        self.band.clear();
        self.band_rows = 0;
        Ok(())
    }

    /// Writes any partial band and the metadata (final shape).
    pub fn finish(mut self, out: &mut Sink) -> Result<(), String> {
        self.flush(out)?;
        let dims = [self.dims[0].as_str(), self.dims[1].as_str()];
        let meta = array_metadata(
            &[self.rows, self.cells],
            &[self.time_chunk, self.cell_chunk],
            &dims,
            self.attributes.clone(),
        );
        out(&join(&self.name, "zarr.json"), meta.as_bytes())
    }
}
