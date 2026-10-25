use serde::{Deserialize, Serialize};

/// Parameters of the M0 toy mechanisms. In later milestones parameters come from the
/// parameter registry with priors and citations (docs/05).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Params {
    /// Mechanism period in ticks (365 = yearly).
    pub period_ticks: u64,
    /// Intrinsic growth rate per period.
    pub growth_rate: f64,
    /// Carrying capacity range per cell (people), drawn once per cell.
    pub capacity_min: u64,
    pub capacity_max: u64,
    /// Bands at least this large may split.
    pub fission_threshold: u64,
    /// Probability per period that an eligible band splits.
    pub fission_probability: f64,
    /// Probability that a daughter band picks a random neighbor instead of the
    /// least crowded one.
    pub exploration_probability: f64,
    /// Bands smaller than this merge with other small bands in their cell.
    pub merge_threshold: u64,
    /// Initial bands and their size.
    pub initial_bands: u32,
    pub initial_weight: u64,
}

impl Default for Params {
    fn default() -> Params {
        Params {
            period_ticks: 365,
            growth_rate: 0.01,
            capacity_min: 20,
            capacity_max: 400,
            fission_threshold: 60,
            fission_probability: 0.2,
            exploration_probability: 0.3,
            merge_threshold: 10,
            initial_bands: 12,
            initial_weight: 30,
        }
    }
}

impl Params {
    pub fn validate(&self) -> Result<(), String> {
        let unit = |name: &str, v: f64| {
            if v.is_finite() && (0.0..=1.0).contains(&v) {
                Ok(())
            } else {
                Err(format!("{name} must be in [0, 1], got {v}"))
            }
        };
        if self.period_ticks == 0 {
            return Err("period_ticks must be > 0".into());
        }
        if !self.growth_rate.is_finite() || self.growth_rate < 0.0 {
            return Err("growth_rate must be finite and >= 0".into());
        }
        if self.capacity_min == 0 || self.capacity_max < self.capacity_min {
            return Err("need 0 < capacity_min <= capacity_max".into());
        }
        if self.fission_threshold < 2 {
            return Err("fission_threshold must be >= 2".into());
        }
        unit("fission_probability", self.fission_probability)?;
        unit("exploration_probability", self.exploration_probability)?;
        Ok(())
    }
}
