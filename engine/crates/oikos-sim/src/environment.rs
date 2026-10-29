//! The environment mechanisms see, derived from forcing packs (docs/04 §4.1-4.2).
//!
//! Each cell's dry land is its coastline land fraction in the current year minus its
//! lake fraction (ADR-0043, ADR-0044). With a climate pack, each cell's carrying
//! capacity for foragers is its dry-land area times a density: a terrestrial part that
//! scales with net primary productivity (the Miami model: the lower of a temperature and
//! a precipitation limit, Lieth 1975), plus an aquatic part where a perennial river, a
//! lake shore or a sea coast is at hand. The aquatic part does not depend on local
//! rain, so river valleys in deserts (the Nile) can feed people (ADR-0047).
//!
//! For cultivation (ADR-0049), each cell also has a wild-cereal suitability from its
//! climate, the people its dry land would feed under full cultivation of domestic crops,
//! and a climate stress: how much its forager capacity has fallen over the stress window.
//! A cell's carrying capacity then blends foraging and cultivation by its bands' effort
//! ([`farmed_capacity`]). Everything here is computed from the packs, the year, the
//! parameters and agent totals, with `oikos_math` functions, so every host gets the same
//! integers.

use crate::inputs::{Inputs, LAKES};
use crate::params::Params;
use oikos_io::pack::{FieldView, Slice};
use oikos_math::{exp, pow};

/// Pack kind and fields of the climate a capacity is computed from.
pub const CLIMATE: (&str, &str, &str) = ("climate", "temp_mean", "precip");
/// Hydrology field giving each cell's largest river discharge (m3/s).
pub const RIVERS: (&str, &str) = ("hydrology", "river_discharge");
/// Geometry field giving each cell's area (km2).
pub const AREA: (&str, &str) = ("geometry", "area_km2");

/// Net primary productivity (g dry matter m-2 yr-1) from annual mean temperature (degC)
/// and annual precipitation (mm), by the Miami model.
pub fn npp_miami(temp_c: f64, precip_mm: f64, p: &Params) -> f64 {
    let by_temp = p.miami_npp_max / (1.0 + exp(p.miami_temp_offset - p.miami_temp_slope * temp_c));
    let by_water = p.miami_npp_max * (1.0 - exp(-p.miami_precip_slope * precip_mm.max(0.0)));
    by_temp.min(by_water)
}

/// Forager population density (people/km2) at productivity `npp`, before aquatic resources.
pub fn forager_density(npp: f64, p: &Params) -> f64 {
    if npp <= 0.0 {
        return 0.0;
    }
    p.forager_density_ref * pow(npp / p.npp_reference, p.npp_elasticity)
}

/// Wild-cereal suitability in [0, 1] (the founder crops' wild progenitors: einkorn,
/// emmer, barley, lentil): linear ramps up from too dry and too cold, down toward wet
/// forest (Harlan and Zohary 1966; Zohary, Hopf and Weiss 2012).
pub fn cereal_suitability(temp_c: f64, precip_mm: f64, p: &Params) -> f64 {
    let ramp = |x: f64, lo: f64, hi: f64| ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    ramp(precip_mm, p.cereal_precip_min, p.cereal_precip_full)
        * (1.0 - ramp(precip_mm, p.cereal_precip_wet, p.cereal_precip_max))
        * ramp(temp_c, p.cereal_temp_min, p.cereal_temp_full)
}

/// Carrying capacity of a cell with forager capacity `forage` and `farm` people fed under
/// full cultivation of domestic crops, whose people spend on average `effort` of their
/// effort cultivating and `crop` (the people-weighted mean of effort times domestic share)
/// on domestic-type crops. Effort moves land use from foraging to fields; cultivated wild
/// crops yield `wild_yield_ratio` of domestic ones. At least one person.
pub fn farmed_capacity(forage: u64, farm: f64, effort: f64, crop: f64, p: &Params) -> u64 {
    let fields = farm * (p.wild_yield_ratio * effort + (1.0 - p.wild_yield_ratio) * crop);
    (oikos_math::finite(forage as f64 * (1.0 - effort) + fields) as u64).max(1)
}

/// Land and lake fractions of each cell in one year.
pub struct Surface<'a> {
    coast: Option<(FieldView<'a>, Slice)>,
    lakes: Option<FieldView<'a>>,
}

impl Surface<'_> {
    pub fn new(inputs: &Inputs, calendar_year: i64) -> Surface<'_> {
        let coast = inputs.pack(crate::inputs::COASTLINE.0).ok().map(|p| {
            (p.field(crate::inputs::COASTLINE.1).expect("validated in from_packs"), p.slice_at(calendar_year))
        });
        Surface { coast, lakes: inputs.field(LAKES.0, LAKES.1).ok() }
    }

    /// Whether any pack restricts where people can live.
    pub fn restricted(&self) -> bool {
        self.coast.is_some() || self.lakes.is_some()
    }

    /// Land fraction (1 without a coastline pack; nodata is sea).
    pub fn land(&self, cell: usize) -> f64 {
        self.coast.as_ref().map_or(Some(1.0), |(f, s)| f.at(*s, cell)).unwrap_or(0.0)
    }

    /// Lake fraction (0 without a hydrology pack).
    pub fn lake(&self, cell: usize) -> f64 {
        self.lakes.as_ref().map_or(Some(0.0), |f| f.get(0, cell)).unwrap_or(0.0)
    }

    /// Dry land fraction: land less lakes.
    pub fn dry(&self, cell: usize) -> f64 {
        (self.land(cell) - self.lake(cell)).max(0.0)
    }

    /// A sea coast or a lake shore: some water, some land.
    pub fn shore(&self, cell: usize) -> bool {
        let (land, lake) = (self.land(cell), self.lake(cell));
        (land > 0.0 && land < 1.0) || (lake > 0.0 && lake < 1.0)
    }
}

/// The climate and land of every cell in one year.
struct Year<'a> {
    temp: FieldView<'a>,
    precip: FieldView<'a>,
    slice: Slice,
    area: FieldView<'a>,
    rivers: Option<FieldView<'a>>,
    surface: Surface<'a>,
}

impl Year<'_> {
    /// None without a climate pack.
    fn new(inputs: &Inputs, calendar_year: i64) -> Option<Year<'_>> {
        let climate = inputs.pack(CLIMATE.0).ok()?;
        Some(Year {
            temp: climate.field(CLIMATE.1).expect("validated in from_packs"),
            precip: climate.field(CLIMATE.2).expect("validated in from_packs"),
            slice: climate.slice_at(calendar_year),
            area: inputs.field(AREA.0, AREA.1).expect("validated in from_packs"),
            rivers: inputs.field(RIVERS.0, RIVERS.1).ok(),
            surface: Surface::new(inputs, calendar_year),
        })
    }

    /// Temperature (degC) and precipitation (mm), if the cell has climate data.
    fn climate(&self, c: usize) -> Option<(f64, f64)> {
        Some((self.temp.at(self.slice, c)?, self.precip.at(self.slice, c)?))
    }

    /// Dry-land area (km2).
    fn dry_area(&self, c: usize) -> f64 {
        self.area.get(0, c).unwrap_or(0.0) * self.surface.dry(c)
    }

    /// Foragers the cell feeds (before rounding).
    fn foragers(&self, c: usize, p: &Params) -> f64 {
        let npp = self.climate(c).map_or(0.0, |(t, w)| npp_miami(t, w, p));
        let river =
            self.rivers.as_ref().and_then(|f| f.get(0, c)).is_some_and(|q| q >= p.perennial_discharge);
        let aquatic =
            if river || self.surface.shore(c) { p.aquatic_bonus * p.forager_density_ref } else { 0.0 };
        oikos_math::finite(self.dry_area(c) * (forager_density(npp, p) + aquatic))
    }
}

/// Writes each cell's forager carrying capacity in `calendar_year` into `out` (at least
/// one person, so growth stays defined). Returns false, leaving `out` alone, without a
/// climate pack.
pub fn capacity_into(inputs: &Inputs, calendar_year: i64, p: &Params, out: &mut [u64]) -> bool {
    let Some(year) = Year::new(inputs, calendar_year) else {
        return false;
    };
    for (c, k) in out.iter_mut().enumerate() {
        *k = (year.foragers(c, p) as u64).max(1);
    }
    true
}

/// Per-cell fields for cultivation in one year (ADR-0049).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Farmland {
    /// Wild-cereal suitability, in [0, 1].
    pub suitability: Vec<f64>,
    /// People fed if all effort went into cultivating fully domestic crops: suitability
    /// times dry-land area times `farm_density`.
    pub farm: Vec<f64>,
    /// Relative decline of forager capacity since `stress_window_years` earlier, in [0, 1].
    pub stress: Vec<f64>,
}

impl Farmland {
    /// The fields for `cells` cells in `calendar_year`; empty without a climate pack (no
    /// cultivation).
    pub fn new(inputs: &Inputs, calendar_year: i64, p: &Params, cells: usize) -> Farmland {
        let mut f = Farmland::default();
        f.update(inputs, calendar_year, p, cells);
        f
    }

    /// [`Farmland::new`] into existing buffers.
    pub fn update(&mut self, inputs: &Inputs, calendar_year: i64, p: &Params, cells: usize) {
        let (Some(now), Some(before)) = (
            Year::new(inputs, calendar_year),
            Year::new(inputs, calendar_year - p.stress_window_years as i64),
        ) else {
            *self = Farmland::default();
            return;
        };
        for v in [&mut self.suitability, &mut self.farm, &mut self.stress] {
            v.resize(cells, 0.0);
        }
        for c in 0..cells {
            let s = now.climate(c).map_or(0.0, |(t, w)| cereal_suitability(t, w, p));
            self.suitability[c] = s;
            self.farm[c] = oikos_math::finite(now.dry_area(c) * s * p.farm_density);
            let (k, k0) = (now.foragers(c, p), before.foragers(c, p));
            self.stress[c] = if k0 > 0.0 { (1.0 - k / k0).clamp(0.0, 1.0) } else { 0.0 };
        }
    }

    /// Whether the forcing supports cultivation.
    pub fn active(&self) -> bool {
        !self.suitability.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn miami_model_known_answers() {
        let p = Params::default();
        // 10 degC, 1000 mm: temperature-limited, 3000 / (1 + e^0.125) = 1406.37.
        assert!((npp_miami(10.0, 1000.0, &p) - 1406.37).abs() < 0.01);
        // 25 degC, 300 mm: water-limited, 3000 (1 - e^-0.1992) = 541.84.
        assert!((npp_miami(25.0, 300.0, &p) - 541.84).abs() < 0.01);
        assert_eq!(npp_miami(20.0, 0.0, &p), 0.0);
        assert!(npp_miami(-20.0, 800.0, &p) < 300.0, "cold limits growth");
    }

    #[test]
    fn density_scales_with_productivity() {
        let p = Params { npp_elasticity: 1.5, ..Params::default() };
        assert!((forager_density(p.npp_reference, &p) - p.forager_density_ref).abs() < 1e-12);
        let ratio = forager_density(2.0 * p.npp_reference, &p) / forager_density(p.npp_reference, &p);
        assert!((ratio - 2f64.powf(1.5)).abs() < 1e-9);
        assert_eq!(forager_density(0.0, &p), 0.0);
    }
}
