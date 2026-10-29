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
    pub capacity_min: u64,
    pub capacity_max: u64,
    pub fission_threshold: u64,
    pub fission_probability: f64,
    pub exploration_probability: f64,
    pub merge_threshold: u64,
    pub initial_bands: u32,
    pub initial_weight: u64,
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
            capacity_min: int("capacity_min")?,
            capacity_max: int("capacity_max")?,
            fission_threshold: int("fission_threshold")?,
            fission_probability: real("fission_probability")?,
            exploration_probability: real("exploration_probability")?,
            merge_threshold: int("merge_threshold")?,
            initial_bands: u32::try_from(int("initial_bands")?).map_err(|e| format!("initial_bands: {e}"))?,
            initial_weight: int("initial_weight")?,
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
        if self.capacity_min > self.capacity_max {
            return Err(format!(
                "capacity_min ({}) must not exceed capacity_max ({})",
                self.capacity_min, self.capacity_max
            ));
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for Params {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Params, D::Error> {
        let overrides = BTreeMap::<String, Value>::deserialize(d)?;
        Params::from_registry(registry(), &overrides).map_err(serde::de::Error::custom)
    }
}
