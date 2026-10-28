//! Forcing packs: environmental inputs (geometry, climate, terrain, soils, hydrology,
//! sea level) as per-cell fields, built offline by the `data/` pipelines and read by
//! every host (docs/03-engine.md §3.8a, ADR-0041).
//!
//! Layout (little-endian):
//! `"OIKOSPK1" | header_len u32 | header (canonical JSON) | zero padding | arrays`,
//! each array 8-byte aligned at the offset its field declares. A pack is identified by
//! the BLAKE3 hash of the whole file, so a work unit names exactly the bytes it needs.
//! All packs of a region carry the hash of the region's ascending H3 cell list; a
//! geometry pack also carries the list itself (`h3`), which must match that hash.
//!
//! Values are stored compactly (integers with `scale`/`offset`, or floats) and read as
//! `f64` with correctly rounded operations only, so every host sees the same numbers.

use serde::{Deserialize, Serialize};
use std::ops::Range;

const MAGIC: &[u8; 8] = b"OIKOSPK1";
pub const FORMAT: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    pub name: String,
    pub resolution: u8,
    pub cells: u32,
    /// [`cells_hash`] of the region's ascending H3 indices.
    pub cells_hash: String,
}

/// BLAKE3 (hex) of H3 indices as little-endian `u64`: the identity of a region's cells.
pub fn cells_hash(h3: &[u64]) -> String {
    let bytes: Vec<u8> = h3.iter().flat_map(|c| c.to_le_bytes()).collect();
    blake3::hash(&bytes).to_hex().to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeRule {
    /// The last slice at or before the requested year.
    Step,
    /// Linear interpolation between the two surrounding slices.
    Linear,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Time {
    pub calendar_year: Vec<i64>,
    pub rule: TimeRule,
}

/// Array dimensions, written as `["cell"]` or `["time", "cell"]` (time-major).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub enum Dims {
    Cell,
    TimeCell,
}

impl TryFrom<Vec<String>> for Dims {
    type Error = String;
    fn try_from(v: Vec<String>) -> Result<Dims, String> {
        match v.iter().map(String::as_str).collect::<Vec<_>>()[..] {
            ["cell"] => Ok(Dims::Cell),
            ["time", "cell"] => Ok(Dims::TimeCell),
            _ => Err(format!("dims must be [\"cell\"] or [\"time\", \"cell\"], got {v:?}")),
        }
    }
}

impl From<Dims> for Vec<String> {
    fn from(d: Dims) -> Vec<String> {
        match d {
            Dims::Cell => vec!["cell".into()],
            Dims::TimeCell => vec!["time".into(), "cell".into()],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub dataset: String,
    pub version: String,
    pub license: String,
    pub citation: String,
    pub manifest_sha256: String,
    /// Notice the dataset's license requires on derived data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
}

/// What a field means; the builder adds its type and placement.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldSpec {
    pub name: String,
    pub unit: String,
    pub description: String,
    pub dims: Dims,
    /// Physical value = stored × scale + offset.
    pub scale: f64,
    pub offset: f64,
    /// Stored value meaning "no data" (e.g. sea cells in a soil field).
    pub nodata: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldDef {
    pub name: String,
    pub unit: String,
    pub description: String,
    pub dims: Dims,
    pub dtype: String,
    /// Physical value = stored × scale_factor + add_offset (CF conventions).
    pub scale_factor: f64,
    pub add_offset: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodata: Option<i64>,
    /// Byte offset of the array in the file (8-byte aligned) and its length.
    pub offset: u64,
    pub len: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Header {
    pub format: u32,
    pub kind: String,
    pub region: Region,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<Time>,
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub builder: serde_json::Value,
}

/// Typed array contents for [`PackBuilder::field`].
#[derive(Clone, Debug, PartialEq)]
pub enum Data {
    U8(Vec<u8>),
    U16(Vec<u16>),
    U32(Vec<u32>),
    U64(Vec<u64>),
    I16(Vec<i16>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
}

macro_rules! le_bytes {
    ($v:expr) => {
        ($v.len(), $v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>())
    };
}

impl Data {
    fn dtype_and_bytes(&self) -> (&'static str, usize, Vec<u8>) {
        let (t, (n, b)) = match self {
            Data::U8(v) => ("u8", le_bytes!(v)),
            Data::U16(v) => ("u16", le_bytes!(v)),
            Data::U32(v) => ("u32", le_bytes!(v)),
            Data::U64(v) => ("u64", le_bytes!(v)),
            Data::I16(v) => ("i16", le_bytes!(v)),
            Data::I32(v) => ("i32", le_bytes!(v)),
            Data::I64(v) => ("i64", le_bytes!(v)),
            Data::F32(v) => ("f32", le_bytes!(v)),
            Data::F64(v) => ("f64", le_bytes!(v)),
        };
        (t, n, b)
    }
}

fn dtype_size(dtype: &str) -> Option<usize> {
    Some(match dtype {
        "u8" => 1,
        "u16" | "i16" => 2,
        "u32" | "i32" | "f32" => 4,
        "u64" | "i64" | "f64" => 8,
        _ => return None,
    })
}

/// Writes packs (tests, and Rust-side pack tools; the `data/` pipelines write the same
/// layout from Python).
#[derive(Debug)]
pub struct PackBuilder {
    kind: String,
    region: Region,
    time: Option<Time>,
    sources: Vec<Source>,
    builder: serde_json::Value,
    fields: Vec<(FieldDef, Vec<u8>)>,
}

impl PackBuilder {
    pub fn new(kind: &str, region: Region) -> PackBuilder {
        PackBuilder {
            kind: kind.into(),
            region,
            time: None,
            sources: Vec::new(),
            builder: serde_json::Value::Null,
            fields: Vec::new(),
        }
    }

    pub fn time(&mut self, calendar_year: Vec<i64>, rule: TimeRule) -> &mut Self {
        self.time = Some(Time { calendar_year, rule });
        self
    }

    pub fn source(&mut self, source: Source) -> &mut Self {
        self.sources.push(source);
        self
    }

    /// Free-form provenance of the tool that built the pack (name, git revision).
    pub fn builder_info(&mut self, info: serde_json::Value) -> &mut Self {
        self.builder = info;
        self
    }

    pub fn field(&mut self, spec: FieldSpec, data: Data) -> Result<&mut Self, String> {
        if self.fields.iter().any(|(f, _)| f.name == spec.name) {
            return Err(format!("duplicate field {}", spec.name));
        }
        let times = match spec.dims {
            Dims::Cell => 1,
            Dims::TimeCell => match &self.time {
                Some(t) => t.calendar_year.len(),
                None => return Err(format!("field {}: set the pack's time slices first", spec.name)),
            },
        };
        let (dtype, n, bytes) = data.dtype_and_bytes();
        let want = times * self.region.cells as usize;
        if n != want {
            return Err(format!("field {}: expected {want} values, got {n}", spec.name));
        }
        let def = FieldDef {
            name: spec.name,
            unit: spec.unit,
            description: spec.description,
            dims: spec.dims,
            dtype: dtype.into(),
            scale_factor: spec.scale,
            add_offset: spec.offset,
            nodata: spec.nodata,
            offset: 0,
            len: bytes.len() as u64,
        };
        self.fields.push((def, bytes));
        Ok(self)
    }

    pub fn build(&self) -> Vec<u8> {
        // Offsets depend on the header length, which depends on the offsets' digits:
        // iterate until the layout is stable (twice in practice).
        let mut start = 0u64;
        loop {
            let mut at = start;
            let fields: Vec<FieldDef> = self
                .fields
                .iter()
                .map(|(f, b)| {
                    at = at.next_multiple_of(8);
                    let def = FieldDef { offset: at, ..f.clone() };
                    at += b.len() as u64;
                    def
                })
                .collect();
            let header = Header {
                format: FORMAT,
                kind: self.kind.clone(),
                region: self.region.clone(),
                time: self.time.clone(),
                fields,
                sources: self.sources.clone(),
                builder: self.builder.clone(),
            };
            let text = canonical_json(&header);
            let data_start = (12 + text.len() as u64).next_multiple_of(8);
            if data_start != start {
                start = data_start;
                continue;
            }
            let mut out = Vec::with_capacity(at as usize);
            out.extend_from_slice(MAGIC);
            out.extend_from_slice(&(text.len() as u32).to_le_bytes());
            out.extend_from_slice(text.as_bytes());
            for (def, (_, bytes)) in header.fields.iter().zip(&self.fields) {
                out.resize(def.offset as usize, 0);
                out.extend_from_slice(bytes);
            }
            return out;
        }
    }
}

/// JSON with sorted keys and no insignificant whitespace.
fn canonical_json(v: &impl Serialize) -> String {
    let value = serde_json::to_value(v).expect("pack header serializes");
    serde_json::to_string(&value).expect("pack header serializes")
}

/// A parsed, validated pack.
pub struct Pack {
    id: String,
    header: Header,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for Pack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pack({} {}, {} bytes)", self.header.kind, self.id, self.bytes.len())
    }
}

/// Where a calendar year falls between time slices: value = (1 - w) × lo + w × hi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slice {
    pub lo: usize,
    pub hi: usize,
    pub w: f64,
}

impl Pack {
    /// Parses and validates a pack; its id is the BLAKE3 hash of `bytes`.
    pub fn parse(bytes: Vec<u8>) -> Result<Pack, String> {
        if bytes.len() < 8 || &bytes[..8] != MAGIC {
            return Err("not an Oikos forcing pack".into());
        }
        let len = bytes.get(8..12).ok_or("pack truncated")?;
        let len = u32::from_le_bytes(len.try_into().unwrap()) as usize;
        let text = bytes.get(12..12usize.saturating_add(len)).ok_or("pack truncated")?;
        let header: Header = serde_json::from_slice(text).map_err(|e| format!("pack header: {e}"))?;
        if header.format != FORMAT {
            return Err(format!("unsupported pack format {}", header.format));
        }
        let data_start = (12 + len as u64).next_multiple_of(8);
        let times = match &header.time {
            Some(t) => {
                if t.calendar_year.is_empty() || t.calendar_year.windows(2).any(|w| w[0] >= w[1]) {
                    return Err("pack time slices must be strictly ascending and non-empty".into());
                }
                t.calendar_year.len()
            }
            None => 0,
        };
        let cells = header.region.cells as u64;
        for f in &header.fields {
            let size =
                dtype_size(&f.dtype).ok_or_else(|| format!("field {}: unknown dtype {}", f.name, f.dtype))?;
            let n = match f.dims {
                Dims::Cell => cells,
                Dims::TimeCell if times == 0 => {
                    return Err(format!("field {} is time-sliced but the pack has no time", f.name));
                }
                Dims::TimeCell => times as u64 * cells,
            };
            if f.len != n * size as u64 {
                return Err(format!("field {}: {} bytes, expected {}", f.name, f.len, n * size as u64));
            }
            if f.offset % 8 != 0 || f.offset < data_start {
                return Err(format!(
                    "field {}: offset {} must be 8-byte aligned after the header",
                    f.name, f.offset
                ));
            }
            if f.offset + f.len > bytes.len() as u64 {
                return Err(format!("field {}: array lies outside the file", f.name));
            }
        }
        let id = blake3::hash(&bytes).to_hex().to_string();
        let pack = Pack { id, header, bytes };
        if pack.header.kind == "geometry" && pack.field("h3").is_err() {
            return Err("geometry pack needs an h3 field".into());
        }
        if let Ok(h3) = pack.field("h3") {
            let ids: Vec<u64> = h3.raw_u64().ok_or("field h3 must be u64 per cell")?;
            if cells_hash(&ids) != pack.header.region.cells_hash {
                return Err("geometry pack: h3 cells do not match the region's cells_hash".into());
            }
        }
        Ok(pack)
    }

    /// [`Pack::parse`], also checking that the bytes are the pack `id` names.
    pub fn open(bytes: Vec<u8>, id: &str) -> Result<Pack, String> {
        let pack = Pack::parse(bytes)?;
        if pack.id != id {
            return Err(format!("pack id mismatch: expected {id}, file is {}", pack.id));
        }
        Ok(pack)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn field(&self, name: &str) -> Result<FieldView<'_>, String> {
        let def = self
            .header
            .fields
            .iter()
            .find(|f| f.name == name)
            .ok_or_else(|| format!("pack has no field {name}"))?;
        let span: Range<usize> = def.offset as usize..(def.offset + def.len) as usize;
        let times = match def.dims {
            Dims::Cell => 1,
            Dims::TimeCell => self.header.time.as_ref().map_or(1, |t| t.calendar_year.len()),
        };
        Ok(FieldView { def, raw: &self.bytes[span], cells: self.header.region.cells as usize, times })
    }

    /// Id, header and per-field value range and nodata count, for inspection
    /// (`oikos pack <file>`).
    pub fn summary(&self) -> serde_json::Value {
        let fields: Vec<serde_json::Value> = self
            .header
            .fields
            .iter()
            .map(|def| {
                let f = self.field(&def.name).expect("header field exists");
                let (mut lo, mut hi, mut nodata) = (f64::INFINITY, f64::NEG_INFINITY, 0u64);
                for t in 0..f.times() {
                    for c in 0..f.cells {
                        match f.get(t, c) {
                            Some(v) => (lo, hi) = (lo.min(v), hi.max(v)),
                            None => nodata += 1,
                        }
                    }
                }
                let (min, max) = if lo <= hi { (Some(lo), Some(hi)) } else { (None, None) };
                serde_json::json!({
                    "name": def.name, "unit": def.unit, "dtype": def.dtype, "dims": def.dims,
                    "min": min, "max": max, "nodata": nodata,
                })
            })
            .collect();
        serde_json::json!({
            "id": self.id, "kind": self.header.kind, "region": self.header.region,
            "time": self.header.time, "fields": fields, "sources": self.header.sources,
            "builder": self.header.builder, "bytes": self.bytes.len(),
        })
    }

    /// The slices around `calendar_year` under the pack's time rule; outside the covered
    /// years the nearest slice holds. Packs without time give slice 0.
    pub fn slice_at(&self, calendar_year: i64) -> Slice {
        let Some(t) = &self.header.time else { return Slice { lo: 0, hi: 0, w: 0.0 } };
        let ys = &t.calendar_year;
        let after = ys.partition_point(|&y| y <= calendar_year);
        if after == 0 {
            return Slice { lo: 0, hi: 0, w: 0.0 };
        }
        let lo = after - 1;
        if lo + 1 == ys.len() || t.rule == TimeRule::Step {
            return Slice { lo, hi: lo, w: 0.0 };
        }
        let w = (calendar_year - ys[lo]) as f64 / (ys[lo + 1] - ys[lo]) as f64;
        Slice { lo, hi: lo + 1, w }
    }
}

/// One field's values.
#[derive(Debug)]
pub struct FieldView<'a> {
    pub def: &'a FieldDef,
    raw: &'a [u8],
    cells: usize,
    times: usize,
}

impl FieldView<'_> {
    pub fn times(&self) -> usize {
        self.times
    }

    /// Stored value at (time slice, cell) as `f64` with scale/offset applied; `None` for
    /// nodata. Static fields ignore `time`.
    pub fn get(&self, time: usize, cell: usize) -> Option<f64> {
        let t = if self.def.dims == Dims::Cell { 0 } else { time };
        assert!(
            t < self.times && cell < self.cells,
            "field {}: ({time}, {cell}) out of range",
            self.def.name
        );
        let i = t * self.cells + cell;
        let (stored, int) = self.stored(i);
        if let (Some(nd), Some(v)) = (self.def.nodata, int)
            && v == nd
        {
            return None;
        }
        Some(stored * self.def.scale_factor + self.def.add_offset)
    }

    /// The value at a [`Slice`]; nodata if either surrounding slice is nodata.
    pub fn at(&self, s: Slice, cell: usize) -> Option<f64> {
        let lo = self.get(s.lo, cell)?;
        if s.hi == s.lo || self.def.dims == Dims::Cell {
            return Some(lo);
        }
        let hi = self.get(s.hi, cell)?;
        Some(lo + (hi - lo) * s.w)
    }

    fn stored(&self, i: usize) -> (f64, Option<i64>) {
        macro_rules! read {
            ($t:ty) => {{
                const N: usize = std::mem::size_of::<$t>();
                <$t>::from_le_bytes(self.raw[i * N..(i + 1) * N].try_into().unwrap())
            }};
        }
        match self.def.dtype.as_str() {
            "u8" => int(read!(u8) as i64),
            "u16" => int(read!(u16) as i64),
            "u32" => int(read!(u32) as i64),
            "u64" => {
                let v = read!(u64);
                (v as f64, i64::try_from(v).ok())
            }
            "i16" => int(read!(i16) as i64),
            "i32" => int(read!(i32) as i64),
            "i64" => int(read!(i64)),
            "f32" => (read!(f32) as f64, None),
            "f64" => (read!(f64), None),
            other => unreachable!("dtype {other} validated at parse"),
        }
    }

    /// The raw values of a `u64` field (e.g. H3 indices).
    pub fn raw_u64(&self) -> Option<Vec<u64>> {
        (self.def.dtype == "u64")
            .then(|| self.raw.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect())
    }
}

fn int(v: i64) -> (f64, Option<i64>) {
    (v as f64, Some(v))
}

#[cfg(test)]
mod tests;
