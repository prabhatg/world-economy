//! The forcing packs a run reads (docs/03-engine.md §3.8a).
//!
//! A run's inputs are named by pack id; the bytes are fetched through the host's `read`
//! and must hash to that id. All packs describe the same region, defined by exactly one
//! geometry pack, and each kind appears once, so a mechanism asks for `("terrain",
//! "elev")` without ambiguity. The sorted ids are part of run identity and are recorded
//! in checkpoints, so a run can only resume with the packs it started with.
//!
//! A `coastline` pack's `land_fraction` decides which cells are habitable in each year
//! (ADR-0043); without one, every cell is.

use oikos_io::pack::{FieldView, Pack};
use serde::{Deserialize, Serialize};

/// Pack kind and field holding each cell's land fraction over time.
pub const COASTLINE: (&str, &str) = ("coastline", "land_fraction");

/// A pack named by id, with where this host finds its bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackRef {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Default)]
pub struct Inputs {
    /// Sorted by id.
    packs: Vec<Pack>,
}

impl Inputs {
    pub fn none() -> Inputs {
        Inputs::default()
    }

    /// Reads each pack through `read`, checks it hashes to its id, then validates the set.
    pub fn load(refs: &[PackRef], read: impl Fn(&str) -> Result<Vec<u8>, String>) -> Result<Inputs, String> {
        let packs = refs
            .iter()
            .map(|r| Pack::open(read(&r.path)?, &r.id).map_err(|e| format!("{}: {e}", r.path)))
            .collect::<Result<Vec<_>, String>>()?;
        Inputs::from_packs(packs)
    }

    /// Validates a set of packs: one geometry pack, unique kinds, one region.
    pub fn from_packs(mut packs: Vec<Pack>) -> Result<Inputs, String> {
        packs.sort_by(|a, b| a.id().cmp(b.id()));
        if packs.is_empty() {
            return Ok(Inputs::none());
        }
        let mut kinds: Vec<&str> = packs.iter().map(|p| p.header().kind.as_str()).collect();
        kinds.sort_unstable();
        if let Some(w) = kinds.windows(2).find(|w| w[0] == w[1]) {
            return Err(format!("forcing packs: more than one {} pack", w[0]));
        }
        let Some(geometry) = packs.iter().find(|p| p.header().kind == "geometry") else {
            return Err("forcing packs need a geometry pack defining the region".into());
        };
        let region = &geometry.header().region;
        for p in &packs {
            if p.header().region.cells_hash != region.cells_hash {
                return Err(format!(
                    "pack {} ({}) is for region {} with a different cells_hash than the geometry pack's region {}",
                    p.id(),
                    p.header().kind,
                    p.header().region.name,
                    region.name
                ));
            }
        }
        let inputs = Inputs { packs };
        if inputs.pack(COASTLINE.0).is_ok() {
            inputs.field(COASTLINE.0, COASTLINE.1)?;
        }
        Ok(inputs)
    }

    pub fn is_empty(&self) -> bool {
        self.packs.is_empty()
    }

    /// Pack ids, sorted.
    pub fn ids(&self) -> Vec<&str> {
        self.packs.iter().map(Pack::id).collect()
    }

    pub fn geometry(&self) -> Option<&Pack> {
        self.packs.iter().find(|p| p.header().kind == "geometry")
    }

    pub fn pack(&self, kind: &str) -> Result<&Pack, String> {
        self.packs
            .iter()
            .find(|p| p.header().kind == kind)
            .ok_or_else(|| format!("no {kind} pack among the inputs"))
    }

    /// Whether each of `out.len()` cells is habitable in `calendar_year`: it has land
    /// (land fraction above zero, interpolated under the pack's time rule), or there is
    /// no coastline pack. Returns whether there is one.
    pub fn habitable_into(&self, calendar_year: i64, out: &mut [bool]) -> bool {
        let Ok(coast) = self.pack(COASTLINE.0) else {
            out.fill(true);
            return false;
        };
        let land = coast.field(COASTLINE.1).expect("validated in from_packs");
        let s = coast.slice_at(calendar_year);
        for (c, h) in out.iter_mut().enumerate() {
            *h = land.at(s, c).is_some_and(|f| f > 0.0);
        }
        true
    }

    /// Field `name` of the pack of `kind`.
    pub fn field(&self, kind: &str, name: &str) -> Result<FieldView<'_>, String> {
        self.pack(kind)?.field(name).map_err(|e| format!("{kind} pack: {e}"))
    }
}
