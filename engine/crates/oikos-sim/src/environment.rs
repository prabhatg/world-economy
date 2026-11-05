//! The environment mechanisms see, derived from forcing packs (docs/04 §4.1-4.2).
//!
//! Each cell's dry land is its coastline land fraction in the current year minus its
//! lake fraction (ADR-0043, ADR-0044). With a climate pack, each cell's carrying
//! capacity for foragers is its dry-land area times a density: a terrestrial part that
//! scales with net primary productivity (the Miami model: the lower of a temperature and
//! a precipitation limit, Lieth 1975), plus an aquatic part where a perennial river, a
//! lake shore or a sea coast is at hand. The aquatic part does not depend on local
//! rain, so river valleys in deserts (the Nile) can feed people (ADR-0047). A
//! `humid_period` pack adds the rain of the African Humid Period that the climate pack's
//! model misses, along a time course its parameters set ([`humid_share`], ADR-0084).
//!
//! For cultivation (ADR-0049), each cell also has a wild-cereal suitability from its
//! climate, the people its dry land would feed under full cultivation of domestic crops,
//! a climate stress: how much its forager capacity has fallen over the stress window, and a
//! climate variability: how much its suitability has swung per century over that window.
//! A cell's carrying capacity then blends foraging and cultivation by its bands' effort
//! ([`farmed_capacity`]), with fields yielding less where they have worn the soil down
//! ([`erode`], [`soil_yield`]). Everything here is computed from the packs, the year, the
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
/// Hydrology field giving the share of each cell its rivers flood at their highest.
pub const FLOODPLAIN: (&str, &str) = ("hydrology", "floodplain_fraction");
/// Pack kind and field holding the share of each cell's land that is upland where wild
/// sheep and goats lived (0 in Africa; #37).
pub const CAPRINES: (&str, &str) = ("caprines", "upland");
/// Geometry field giving each cell's area (km2).
pub const AREA: (&str, &str) = ("geometry", "area_km2");
/// Pack kind and field holding the rain the African Humid Period adds at its height
/// (mm/year; ADR-0084).
pub const HUMID: (&str, &str) = ("humid_period", "precip_anomaly");

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

/// Share in [0, 1] of the African Humid Period's rain in `calendar_year` (ADR-0084): it
/// rises over `humid_period_transition_years` centered on `humid_period_onset_bp` and
/// falls over as many centered on `humid_period_end_bp` (years before 1950), linearly.
pub fn humid_share(calendar_year: i64, p: &Params) -> f64 {
    let bp = (1950 - calendar_year) as f64;
    let ramp = |x: f64| (x / p.humid_period_transition_years as f64 + 0.5).clamp(0.0, 1.0);
    ramp(p.humid_period_onset_bp as f64 - bp) * ramp(bp - p.humid_period_end_bp as f64)
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

/// Carrying capacity of a cell with forager capacity `forage`, `farm` people fed under
/// full cultivation of domestic crops on its rain-fed land and `flood` on its floodplain,
/// whose people spend on average `effort` of their effort cultivating and `crop` (the
/// people-weighted mean of effort times domestic share) on domestic-type crops. Effort
/// moves land use from foraging to fields; cultivated wild crops yield `wild_yield_ratio`
/// of domestic ones on rain-fed land and nothing on the floodplain, where no wild cereals
/// grow (ADR-0085). At least one person.
pub fn farmed_capacity(forage: u64, farm: f64, flood: f64, effort: f64, crop: f64, p: &Params) -> u64 {
    let land = Land { forage, farm, flood, ..Land::default() };
    capacity_of(&land, &Practice { effort, crop, ..Practice::default() }, p)
}

/// What a cell's land feeds under each use: people per cell.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Land {
    pub forage: u64,
    /// Full cultivation of domestic crops on rain-fed land, and on the floodplain.
    pub farm: f64,
    pub flood: f64,
    /// Full herding of domestic sheep and goats on the cell's wild herds' range, and on all
    /// its pasture, which holds the range (#37).
    pub herd_range: f64,
    pub herd_pasture: f64,
}

/// A cell's people's mean practice: effort cultivating and effort times the crop's
/// domestic share, effort herding and effort times the herd's domestic share.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Practice {
    pub effort: f64,
    pub crop: f64,
    pub herding: f64,
    pub herd_crop: f64,
}

/// Carrying capacity of a cell (at least one person): foraging with the effort left from
/// cultivating and herding, plus fields ([`farmed_capacity`]) and herds. Managed wild herds
/// yield `wild_herd_ratio` of domestic ones and live only on their range; domestic herds
/// graze all the pasture (#37).
pub fn capacity_of(land: &Land, practice: &Practice, p: &Params) -> u64 {
    capacity_at(land, practice, (p.wild_yield_ratio, p.wild_herd_ratio))
}

/// [`capacity_of`] at the yields of wild crops and wild herds relative to domestic ones,
/// `(wild_yield_ratio, wild_herd_ratio)`.
pub fn capacity_at(land: &Land, practice: &Practice, (w, h): (f64, f64)) -> u64 {
    let Practice { effort, crop, herding, herd_crop } = *practice;
    let fields = land.farm * (w * effort + (1.0 - w) * crop) + land.flood * crop;
    let herds = land.herd_range * (h * herding + (1.0 - h) * herd_crop)
        + (land.herd_pasture - land.herd_range).max(0.0) * herd_crop;
    let foraging = land.forage as f64 * (1.0 - effort - herding).max(0.0);
    (oikos_math::finite(foraging + fields + herds) as u64).max(1)
}

/// A cell's soil share (1 = its original erodible soil) after `years` with its people
/// spending `effort` cultivating: fields erode it at `soil_erosion_rate` times effort, a
/// steady loss of depth (Montgomery 2007), while soil re-forms at `soil_formation_rate`
/// times what is missing. Between 0 (bare rock) and 1.
pub fn erode(soil: f64, effort: f64, years: f64, p: &Params) -> f64 {
    let loss = p.soil_erosion_rate * effort * years;
    let gain = p.soil_formation_rate * (1.0 - soil) * years;
    oikos_math::finite(soil - loss + gain).clamp(0.0, 1.0)
}

/// Share of full field yields on soil `soil`: whole while roots have at least
/// `soil_critical` of the soil, then linear to nothing on bare rock (Lal 1998).
pub fn soil_yield(soil: f64, p: &Params) -> f64 {
    (soil / p.soil_critical).min(1.0)
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
    /// The Humid Period's anomaly and the multiple of it this year, if it adds rain.
    humid: Option<(FieldView<'a>, f64)>,
    slice: Slice,
    area: FieldView<'a>,
    rivers: Option<FieldView<'a>>,
    floodplain: Option<FieldView<'a>>,
    upland: Option<FieldView<'a>>,
    surface: Surface<'a>,
}

impl Year<'_> {
    /// None without a climate pack.
    fn new<'a>(inputs: &'a Inputs, calendar_year: i64, p: &Params) -> Option<Year<'a>> {
        let climate = inputs.pack(CLIMATE.0).ok()?;
        let wet = p.humid_period_scale * humid_share(calendar_year, p);
        Some(Year {
            temp: climate.field(CLIMATE.1).expect("validated in from_packs"),
            precip: climate.field(CLIMATE.2).expect("validated in from_packs"),
            humid: inputs.field(HUMID.0, HUMID.1).ok().filter(|_| wet > 0.0).map(|f| (f, wet)),
            slice: climate.slice_at(calendar_year),
            area: inputs.field(AREA.0, AREA.1).expect("validated in from_packs"),
            rivers: inputs.field(RIVERS.0, RIVERS.1).ok(),
            floodplain: inputs.field(FLOODPLAIN.0, FLOODPLAIN.1).ok(),
            upland: inputs.field(CAPRINES.0, CAPRINES.1).ok(),
            surface: Surface::new(inputs, calendar_year),
        })
    }

    /// Temperature (degC) and precipitation (mm, the Humid Period's included), if the
    /// cell has climate data.
    fn climate(&self, c: usize) -> Option<(f64, f64)> {
        let precip = self.precip.at(self.slice, c)?;
        let humid = self.humid.as_ref().and_then(|(f, wet)| Some(wet * f.get(0, c)?)).unwrap_or(0.0);
        Some((self.temp.at(self.slice, c)?, precip + humid))
    }

    /// Dry-land area (km2).
    fn dry_area(&self, c: usize) -> f64 {
        self.area.get(0, c).unwrap_or(0.0) * self.surface.dry(c)
    }

    /// The cell's floodplain fields as a share of its dry land at full yield (ADR-0085):
    /// the share its rivers flood, times what floods grow there beyond what rain already
    /// does at wild-cereal suitability `suitability` (crops need warmth, not rain, there).
    fn floodplain(&self, c: usize, temp_c: f64, suitability: f64, p: &Params) -> f64 {
        let Some(share) = self.floodplain.as_ref().and_then(|f| f.get(0, c)) else {
            return 0.0;
        };
        let warmth =
            ((temp_c - p.cereal_temp_min) / (p.cereal_temp_full - p.cereal_temp_min)).clamp(0.0, 1.0);
        oikos_math::finite(share.clamp(0.0, 1.0) * (warmth * p.floodplain_yield - suitability).max(0.0))
    }

    /// The cell's (range, pasture) for sheep and goats as shares of its dry land (#37):
    /// pasture where the rain grows enough grazing (`pasture_precip_min` to
    /// `pasture_precip_full` mm), and of it the range of wild herds, its upland share.
    fn grazing(&self, c: usize, precip_mm: f64, p: &Params) -> (f64, f64) {
        let ramp = (precip_mm - p.pasture_precip_min) / (p.pasture_precip_full - p.pasture_precip_min);
        let pasture = oikos_math::finite(ramp).clamp(0.0, 1.0);
        let upland = self.upland.as_ref().and_then(|f| f.get(0, c)).unwrap_or(0.0).clamp(0.0, 1.0);
        (upland * pasture, pasture)
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
    let Some(year) = Year::new(inputs, calendar_year, p) else {
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
    /// times dry-land area times `farm_density`, times the soil's yield ([`soil_yield`],
    /// applied by the world, which keeps the soil).
    pub farm: Vec<f64>,
    /// Floodplain fields beyond what rain grows, as a share of dry land at full yield
    /// (ADR-0085): sown only with domestic crops, so it invites only bands that have them.
    pub floodplain: Vec<f64>,
    /// People fed if all effort went into cultivating fully domestic crops on the
    /// floodplain: `floodplain` times dry-land area times `farm_density`. Floods renew its
    /// soil, so it does not wear down.
    pub flood: Vec<f64>,
    /// Range of wild sheep and goats, and pasture for domestic ones, as shares of dry land
    /// (#37); without a caprines pack, no range, so nobody starts herding.
    pub range: Vec<f64>,
    pub pasture: Vec<f64>,
    /// People fed if all effort went into herding domestic sheep and goats on the range,
    /// and on all the pasture: those shares times dry-land area times `herd_density`.
    pub herd_range: Vec<f64>,
    pub herd_pasture: Vec<f64>,
    /// Relative decline of forager capacity since `stress_window_years` earlier, in [0, 1].
    pub stress: Vec<f64>,
    /// Climate variability: the mean absolute change of suitability per century over the
    /// stress window (at least one century), between the century boundaries before the year.
    pub variability: Vec<f64>,
    /// What `variability` was computed for: it changes only at century boundaries.
    variability_key: Option<VariabilityKey>,
}

/// The century, window and suitability parameters a variability field depends on.
#[derive(Clone, Debug, PartialEq)]
struct VariabilityKey {
    anchor: i64,
    centuries: i64,
    suitability: [u64; 6],
    humid: [u64; 4],
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
            Year::new(inputs, calendar_year, p),
            Year::new(inputs, calendar_year - p.stress_window_years as i64, p),
        ) else {
            *self = Farmland::default();
            return;
        };
        for v in [
            &mut self.suitability,
            &mut self.farm,
            &mut self.floodplain,
            &mut self.flood,
            &mut self.range,
            &mut self.pasture,
            &mut self.herd_range,
            &mut self.herd_pasture,
            &mut self.stress,
        ] {
            v.resize(cells, 0.0);
        }
        self.update_variability(inputs, calendar_year, p, cells);
        for c in 0..cells {
            let climate = now.climate(c);
            let s = climate.map_or(0.0, |(t, w)| cereal_suitability(t, w, p));
            self.suitability[c] = s;
            self.farm[c] = oikos_math::finite(now.dry_area(c) * s * p.farm_density);
            self.floodplain[c] = climate.map_or(0.0, |(t, _)| now.floodplain(c, t, s, p));
            self.flood[c] = oikos_math::finite(now.dry_area(c) * self.floodplain[c] * p.farm_density);
            let (range, pasture) = climate.map_or((0.0, 0.0), |(_, w)| now.grazing(c, w, p));
            self.range[c] = range;
            self.pasture[c] = pasture;
            self.herd_range[c] = oikos_math::finite(now.dry_area(c) * range * p.herd_density);
            self.herd_pasture[c] = oikos_math::finite(now.dry_area(c) * pasture * p.herd_density);
            let (k, k0) = (now.foragers(c, p), before.foragers(c, p));
            self.stress[c] = if k0 > 0.0 { (1.0 - k / k0).clamp(0.0, 1.0) } else { 0.0 };
        }
    }

    /// Variability is measured between century boundaries (the climate packs' native
    /// step): between them the climate is interpolated, which would hide swings. So it
    /// is recomputed only when the century, the window, the suitability or the Humid Period's
    /// parameters change.
    fn update_variability(&mut self, inputs: &Inputs, calendar_year: i64, p: &Params, cells: usize) {
        let key = VariabilityKey {
            anchor: calendar_year.div_euclid(100) * 100,
            centuries: (p.stress_window_years / 100).max(1) as i64,
            suitability: [
                p.cereal_precip_min,
                p.cereal_precip_full,
                p.cereal_precip_wet,
                p.cereal_precip_max,
                p.cereal_temp_min,
                p.cereal_temp_full,
            ]
            .map(f64::to_bits),
            humid: [
                p.humid_period_scale.to_bits(),
                p.humid_period_onset_bp,
                p.humid_period_end_bp,
                p.humid_period_transition_years,
            ],
        };
        if self.variability_key.as_ref() == Some(&key) && self.variability.len() == cells {
            return;
        }
        let at = |i: i64| Year::new(inputs, key.anchor - 100 * i, p).expect("the climate pack is present");
        let years: Vec<Year<'_>> = (0..=key.centuries).map(at).collect();
        self.variability.resize(cells, 0.0);
        for c in 0..cells {
            let suit = |y: &Year<'_>| y.climate(c).map_or(0.0, |(t, w)| cereal_suitability(t, w, p));
            let change: f64 = years.windows(2).map(|w| (suit(&w[0]) - suit(&w[1])).abs()).sum();
            self.variability[c] = oikos_math::finite(change / key.centuries as f64);
        }
        self.variability_key = Some(key);
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
