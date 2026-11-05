//! Engine parameters, built from and checked against the parameter registry
//! (`engine/params/m0-toy.toml`, see `oikos-params`).

use oikos_params::{Registry, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// The registry for the current model, embedded at build time so every host runs the
/// same parameter definitions.
pub fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        Registry::from_toml(include_str!("../../../params/m0-toy.toml")).expect("embedded registry is valid")
    })
}

/// Parameters of the engine's mechanisms. Field names are registry keys; units, ranges,
/// priors and sources live in the registry, not here.
///
/// Deserializing (work units, checkpoints) reads the given keys as overrides of the
/// registry defaults, so partial parameter sets are allowed and every value is checked.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Params {
    pub period_ticks: u64,
    pub growth_rate: f64,
    pub sedentary_growth_rate: f64,
    pub fertility: f64,
    pub sedentary_fertility: f64,
    pub capacity_min: u64,
    pub capacity_max: u64,
    pub fission_threshold: u64,
    pub village_fission_size: u64,
    pub aggregation_rate: f64,
    pub settlement_integration: f64,
    pub settlement_hierarchy: f64,
    pub wealth_transmission: f64,
    pub wealth_shock: f64,
    pub landed_wealth_transmission: f64,
    pub landed_wealth_shock: f64,
    pub pastoral_wealth_transmission: f64,
    pub pastoral_wealth_shock: f64,
    pub generation_years: f64,
    pub disease_threshold: f64,
    pub mobile_contact: f64,
    pub disease_mortality: f64,
    pub neighbor_learning: f64,
    pub success_bias: f64,
    pub fission_probability: f64,
    pub exploration_probability: f64,
    pub merge_threshold: u64,
    pub initial_bands: u32,
    pub initial_weight: u64,
    pub initial_occupancy: f64,
    pub capacity_period_ticks: u64,
    pub miami_npp_max: f64,
    pub miami_temp_offset: f64,
    pub miami_temp_slope: f64,
    pub miami_precip_slope: f64,
    pub npp_reference: f64,
    pub forager_density_ref: f64,
    pub npp_elasticity: f64,
    pub aquatic_bonus: f64,
    pub perennial_discharge: f64,
    pub humid_period_scale: f64,
    pub floodplain_yield: f64,
    pub herding_weight: f64,
    pub herd_density: f64,
    pub wild_herd_ratio: f64,
    pub herd_selection: f64,
    pub herd_introgression: f64,
    pub herd_wild_frequency: f64,
    pub pasture_precip_min: f64,
    pub pasture_precip_full: f64,
    pub humid_period_onset_bp: u64,
    pub humid_period_end_bp: u64,
    pub humid_period_transition_years: u64,
    pub dispersal_crowding: f64,
    pub dispersal_probability: f64,
    pub dispersal_fraction: f64,
    pub cultivation_crowding: f64,
    pub cultivation_pressure_weight: f64,
    pub cultivation_stress_weight: f64,
    pub cultivation_abundance_weight: f64,
    pub cultivation_variability_weight: f64,
    pub stress_window_years: u64,
    pub adoption_rate: f64,
    pub innovation_share: f64,
    pub selection_coefficient: f64,
    pub domestic_wild_frequency: f64,
    pub introgression_rate: f64,
    pub farm_density: f64,
    pub wild_yield_ratio: f64,
    pub soil_erosion_rate: f64,
    pub soil_formation_rate: f64,
    pub soil_critical: f64,
    pub obsidian_loss: f64,
    pub cereal_precip_min: f64,
    pub cereal_precip_full: f64,
    pub cereal_precip_wet: f64,
    pub cereal_precip_max: f64,
    pub cereal_temp_min: f64,
    pub cereal_temp_full: f64,
}

impl Default for Params {
    fn default() -> Params {
        Params::from_registry(registry(), &BTreeMap::new()).expect("registry defaults are valid")
    }
}

impl Params {
    /// Registry defaults with `overrides` applied; every value is range-checked.
    pub fn from_registry(registry: &Registry, overrides: &BTreeMap<String, Value>) -> Result<Params, String> {
        let v = registry.resolve(overrides)?;
        let int = |k: &str| match v.get(k) {
            Some(Value::Int(i)) => Ok(*i),
            other => {
                Err(format!("registry {}: {k} must be an integer parameter, found {other:?}", registry.name))
            }
        };
        let real = |k: &str| match v.get(k) {
            Some(Value::Real(r)) => Ok(*r),
            other => {
                Err(format!("registry {}: {k} must be a real parameter, found {other:?}", registry.name))
            }
        };
        let params = Params {
            period_ticks: int("period_ticks")?,
            growth_rate: real("growth_rate")?,
            sedentary_growth_rate: real("sedentary_growth_rate")?,
            fertility: real("fertility")?,
            sedentary_fertility: real("sedentary_fertility")?,
            capacity_min: int("capacity_min")?,
            capacity_max: int("capacity_max")?,
            fission_threshold: int("fission_threshold")?,
            village_fission_size: int("village_fission_size")?,
            aggregation_rate: real("aggregation_rate")?,
            settlement_integration: real("settlement_integration")?,
            settlement_hierarchy: real("settlement_hierarchy")?,
            wealth_transmission: real("wealth_transmission")?,
            wealth_shock: real("wealth_shock")?,
            landed_wealth_transmission: real("landed_wealth_transmission")?,
            landed_wealth_shock: real("landed_wealth_shock")?,
            pastoral_wealth_transmission: real("pastoral_wealth_transmission")?,
            pastoral_wealth_shock: real("pastoral_wealth_shock")?,
            generation_years: real("generation_years")?,
            disease_threshold: real("disease_threshold")?,
            mobile_contact: real("mobile_contact")?,
            disease_mortality: real("disease_mortality")?,
            neighbor_learning: real("neighbor_learning")?,
            success_bias: real("success_bias")?,
            fission_probability: real("fission_probability")?,
            exploration_probability: real("exploration_probability")?,
            merge_threshold: int("merge_threshold")?,
            initial_bands: u32::try_from(int("initial_bands")?).map_err(|e| format!("initial_bands: {e}"))?,
            initial_weight: int("initial_weight")?,
            initial_occupancy: real("initial_occupancy")?,
            capacity_period_ticks: int("capacity_period_ticks")?,
            miami_npp_max: real("miami_npp_max")?,
            miami_temp_offset: real("miami_temp_offset")?,
            miami_temp_slope: real("miami_temp_slope")?,
            miami_precip_slope: real("miami_precip_slope")?,
            npp_reference: real("npp_reference")?,
            forager_density_ref: real("forager_density_ref")?,
            npp_elasticity: real("npp_elasticity")?,
            aquatic_bonus: real("aquatic_bonus")?,
            perennial_discharge: real("perennial_discharge")?,
            humid_period_scale: real("humid_period_scale")?,
            floodplain_yield: real("floodplain_yield")?,
            herding_weight: real("herding_weight")?,
            herd_density: real("herd_density")?,
            wild_herd_ratio: real("wild_herd_ratio")?,
            herd_selection: real("herd_selection")?,
            herd_introgression: real("herd_introgression")?,
            herd_wild_frequency: real("herd_wild_frequency")?,
            pasture_precip_min: real("pasture_precip_min")?,
            pasture_precip_full: real("pasture_precip_full")?,
            humid_period_onset_bp: int("humid_period_onset_bp")?,
            humid_period_end_bp: int("humid_period_end_bp")?,
            humid_period_transition_years: int("humid_period_transition_years")?,
            dispersal_crowding: real("dispersal_crowding")?,
            dispersal_probability: real("dispersal_probability")?,
            dispersal_fraction: real("dispersal_fraction")?,
            cultivation_crowding: real("cultivation_crowding")?,
            cultivation_pressure_weight: real("cultivation_pressure_weight")?,
            cultivation_stress_weight: real("cultivation_stress_weight")?,
            cultivation_abundance_weight: real("cultivation_abundance_weight")?,
            cultivation_variability_weight: real("cultivation_variability_weight")?,
            stress_window_years: int("stress_window_years")?,
            adoption_rate: real("adoption_rate")?,
            innovation_share: real("innovation_share")?,
            selection_coefficient: real("selection_coefficient")?,
            domestic_wild_frequency: real("domestic_wild_frequency")?,
            introgression_rate: real("introgression_rate")?,
            farm_density: real("farm_density")?,
            wild_yield_ratio: real("wild_yield_ratio")?,
            soil_erosion_rate: real("soil_erosion_rate")?,
            soil_formation_rate: real("soil_formation_rate")?,
            soil_critical: real("soil_critical")?,
            obsidian_loss: real("obsidian_loss")?,
            cereal_precip_min: real("cereal_precip_min")?,
            cereal_precip_full: real("cereal_precip_full")?,
            cereal_precip_wet: real("cereal_precip_wet")?,
            cereal_precip_max: real("cereal_precip_max")?,
            cereal_temp_min: real("cereal_temp_min")?,
            cereal_temp_full: real("cereal_temp_full")?,
        };
        params.validate()?;
        Ok(params)
    }

    /// Checks every field against the registry, plus relations between fields.
    pub fn validate(&self) -> Result<(), String> {
        let serde_json::Value::Object(fields) = serde_json::to_value(self).expect("params serialize") else {
            unreachable!("params serialize to an object")
        };
        for (key, v) in &fields {
            let value = match v.as_u64() {
                Some(i) if !v.is_f64() => Value::Int(i),
                _ => Value::Real(v.as_f64().expect("numeric parameter")),
            };
            registry().check(key, value)?;
        }
        let p =
            (self.cereal_precip_min, self.cereal_precip_full, self.cereal_precip_wet, self.cereal_precip_max);
        if !(p.0 < p.1 && p.1 <= p.2 && p.2 < p.3) {
            return Err(format!(
                "cereal precipitation limits must rise: min {} < full {} <= wet {} < max {}",
                p.0, p.1, p.2, p.3
            ));
        }
        if self.cereal_temp_min >= self.cereal_temp_full {
            return Err(format!(
                "cereal_temp_min ({}) must be below cereal_temp_full ({})",
                self.cereal_temp_min, self.cereal_temp_full
            ));
        }
        for (key, beta) in [
            ("wealth_transmission", self.wealth_transmission),
            ("landed_wealth_transmission", self.landed_wealth_transmission),
            ("pastoral_wealth_transmission", self.pastoral_wealth_transmission),
        ] {
            if beta >= 1.0 {
                return Err(format!(
                    "{key} ({beta}) must be below 1: with every generation keeping all of its parents' \
                     inequality, inequality grows without bound (ADR-0074)"
                ));
            }
        }
        let (onset, end, t) =
            (self.humid_period_onset_bp, self.humid_period_end_bp, self.humid_period_transition_years);
        if onset < end + t {
            return Err(format!(
                "humid_period_onset_bp ({onset}) must be at least humid_period_end_bp ({end}) plus \
                 humid_period_transition_years ({t}) earlier: the Humid Period begins before it ends, \
                 and is fully humid between its transitions (ADR-0084)"
            ));
        }
        if self.pasture_precip_min >= self.pasture_precip_full {
            return Err(format!(
                "pasture_precip_min ({}) must be below pasture_precip_full ({})",
                self.pasture_precip_min, self.pasture_precip_full
            ));
        }
        if self.capacity_min > self.capacity_max {
            return Err(format!(
                "capacity_min ({}) must not exceed capacity_max ({})",
                self.capacity_min, self.capacity_max
            ));
        }
        Ok(())
    }
}

/// How a checkpoint written before a parameter existed restores it: as the model it was
/// written with, whatever the registry default has since been calibrated to.
#[derive(Clone, Copy, Debug)]
pub enum Restore {
    /// This value: the nested hypothesis switched off.
    Value(f64),
    /// The value (stored or restored) of another parameter.
    SameAs(&'static str),
    /// The registry default: without the parameters restored before it, it changes nothing.
    Inert,
}

/// Parameters the oldest restorable checkpoints store (format `OIKOSCK2`, ADR-0049).
/// Checkpoints store every parameter of their engine, so one lacking a parameter predates
/// it.
pub const IN_FIRST_CHECKPOINTS: &[&str] = &[
    "adoption_rate",
    "aquatic_bonus",
    "capacity_max",
    "capacity_min",
    "capacity_period_ticks",
    "cereal_precip_full",
    "cereal_precip_max",
    "cereal_precip_min",
    "cereal_precip_wet",
    "cereal_temp_full",
    "cereal_temp_min",
    "cultivation_abundance_weight",
    "cultivation_crowding",
    "cultivation_pressure_weight",
    "cultivation_stress_weight",
    "dispersal_crowding",
    "dispersal_fraction",
    "dispersal_probability",
    "domestic_wild_frequency",
    "exploration_probability",
    "farm_density",
    "fission_probability",
    "fission_threshold",
    "forager_density_ref",
    "growth_rate",
    "initial_bands",
    "initial_occupancy",
    "initial_weight",
    "introgression_rate",
    "merge_threshold",
    "miami_npp_max",
    "miami_precip_slope",
    "miami_temp_offset",
    "miami_temp_slope",
    "npp_elasticity",
    "npp_reference",
    "perennial_discharge",
    "period_ticks",
    "selection_coefficient",
    "stress_window_years",
    "wild_yield_ratio",
];

/// Parameters added since [`IN_FIRST_CHECKPOINTS`], in the order they were added, with how
/// a checkpoint lacking them restores (#31). Every new parameter needs a rule here; a
/// test enforces it.
pub const ADDED_SINCE_CHECKPOINTS: &[(&str, Restore)] = &[
    // ADR-0053: every band grew at the forager rate.
    ("sedentary_growth_rate", Restore::SameAs("growth_rate")),
    // ADR-0054: the stable-climate hypothesis switched off.
    ("cultivation_variability_weight", Restore::Value(0.0)),
    // ADR-0055: no erosion, so soil stays full (as OIKOSCK2 checkpoints restore it) and
    // its formation and critical share change nothing.
    ("soil_erosion_rate", Restore::Value(0.0)),
    ("soil_formation_rate", Restore::Inert),
    ("soil_critical", Restore::Inert),
    // ADR-0060: obsidian is a tracer that never changes history.
    ("obsidian_loss", Restore::Inert),
    // ADR-0065: individual learning.
    ("innovation_share", Restore::Value(1.0)),
    // ADR-0068: births and deaths not split (fertility 0 is the net change alone, rounded
    // once, as before), and no fertility transition.
    ("fertility", Restore::Value(0.0)),
    ("sedentary_fertility", Restore::SameAs("fertility")),
    // ADR-0070: no villages, the fission before them.
    ("village_fission_size", Restore::Value(0.0)),
    // ADR-0071: no aggregation.
    ("aggregation_rate", Restore::Value(0.0)),
    // ADR-0074: wealth inequality is a trait nothing else reads yet.
    ("wealth_transmission", Restore::Inert),
    ("wealth_shock", Restore::Inert),
    ("landed_wealth_transmission", Restore::Inert),
    ("landed_wealth_shock", Restore::Inert),
    ("generation_years", Restore::Inert),
    // ADR-0075: no disease.
    ("disease_mortality", Restore::Value(0.0)),
    ("disease_threshold", Restore::Inert),
    ("mobile_contact", Restore::Inert),
    // ADR-0076: learning from the own cell only, by how many do it.
    ("neighbor_learning", Restore::Value(0.0)),
    ("success_bias", Restore::Value(0.0)),
    // ADR-0082: every settlement is one group.
    ("settlement_integration", Restore::Value(1.0)),
    ("settlement_hierarchy", Restore::Value(0.0)),
    // ADR-0084: the forcing's climate alone (no humid_period pack was read before it).
    ("humid_period_scale", Restore::Value(0.0)),
    ("humid_period_onset_bp", Restore::Inert),
    ("humid_period_end_bp", Restore::Inert),
    ("humid_period_transition_years", Restore::Inert),
    // ADR-0085: no floodplain fields.
    ("floodplain_yield", Restore::Value(0.0)),
    // #37: nobody herds.
    ("herding_weight", Restore::Value(0.0)),
    ("herd_density", Restore::Inert),
    ("wild_herd_ratio", Restore::Inert),
    ("herd_selection", Restore::Inert),
    ("herd_introgression", Restore::Inert),
    ("herd_wild_frequency", Restore::Inert),
    ("pasture_precip_min", Restore::Inert),
    ("pasture_precip_full", Restore::Inert),
    // #45: herds pass on what foragers' wealth does (after the parameters they follow).
    ("pastoral_wealth_transmission", Restore::SameAs("wealth_transmission")),
    ("pastoral_wealth_shock", Restore::SameAs("wealth_shock")),
];

impl Params {
    /// Parameters stored in a checkpoint (JSON): as a work unit's overrides, except that
    /// one lacking a parameter added since restores it by its [`Restore`] rule.
    pub fn from_checkpoint(json: &[u8]) -> Result<Params, String> {
        let mut overrides: BTreeMap<String, Value> =
            serde_json::from_slice(json).map_err(|e| e.to_string())?;
        for &(k, rule) in ADDED_SINCE_CHECKPOINTS {
            if overrides.contains_key(k) {
                continue;
            }
            let v = match rule {
                Restore::Value(v) => match registry().get(k).map(|d| d.default) {
                    Some(Value::Int(_)) => Value::Int(v as u64),
                    _ => Value::Real(v),
                },
                Restore::SameAs(other) => overrides
                    .get(other)
                    .copied()
                    .or_else(|| registry().get(other).map(|d| d.default))
                    .ok_or_else(|| format!("restore rule for {k}: no parameter {other}"))?,
                Restore::Inert => continue,
            };
            overrides.insert(k.to_string(), v);
        }
        Params::from_registry(registry(), &overrides)
    }
}

impl<'de> Deserialize<'de> for Params {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Params, D::Error> {
        let overrides = BTreeMap::<String, Value>::deserialize(d)?;
        Params::from_registry(registry(), &overrides).map_err(serde::de::Error::custom)
    }
}
