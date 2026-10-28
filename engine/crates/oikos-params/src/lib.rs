//! Parameter registry.
//!
//! Every model parameter is declared once, in a versioned TOML file, with its unit,
//! valid range, prior for calibration, description and sources. The engine builds its
//! parameters from the registry and checks every override against it, and calibration
//! tools read the same file (exported as JSON), so a number can never enter a run
//! without a unit, a range and a stated origin (docs/05-calibration-assimilation.md).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A parameter value. Integers are kept distinct so counts never pass through floats.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Int(u64),
    Real(f64),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{v}"),
            Value::Real(v) => write!(f, "{v}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Kind {
    Real { min: f64, max: f64 },
    Integer { min: u64, max: u64 },
    Probability,
}

/// Prior distribution used by calibration (docs/05 §5.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "dist", rename_all = "snake_case", deny_unknown_fields)]
pub enum Prior {
    /// Not calibrated; always the default.
    Fixed,
    Uniform {
        lo: f64,
        hi: f64,
    },
    LogUniform {
        lo: f64,
        hi: f64,
    },
    Normal {
        mean: f64,
        sd: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Engine-test value with no empirical claim; requires a `note`.
    Placeholder,
    /// Taken from the literature; requires at least one source.
    Literature,
    /// Fitted to data by calibration; requires at least one source (the calibration run).
    Calibrated,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub citation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParamDef {
    pub group: String,
    pub unit: String,
    pub description: String,
    pub kind: Kind,
    pub default: Value,
    pub prior: Prior,
    pub status: Status,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Source>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub name: String,
    pub version: u32,
    #[serde(default)]
    params: BTreeMap<String, ParamDef>,
}

impl Registry {
    pub fn from_toml(src: &str) -> Result<Registry, String> {
        let r: Registry = toml::from_str(src).map_err(|e| format!("parameter registry: {e}"))?;
        if r.params.is_empty() {
            return Err(format!("parameter registry {} has no parameters", r.name));
        }
        for (key, def) in &r.params {
            def.validate().map_err(|e| format!("parameter {key}: {e}"))?;
        }
        Ok(r)
    }

    pub fn get(&self, key: &str) -> Option<&ParamDef> {
        self.params.get(key)
    }

    /// Keys in sorted order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.params.keys().map(String::as_str)
    }

    /// Checks one value against its definition; returns it normalized to the kind.
    pub fn check(&self, key: &str, value: Value) -> Result<Value, String> {
        let def = self.params.get(key).ok_or_else(|| format!("unknown parameter {key}"))?;
        def.check(value).map_err(|e| format!("parameter {key}: {e}"))
    }

    /// Defaults with `overrides` applied, every value checked.
    pub fn resolve(&self, overrides: &BTreeMap<String, Value>) -> Result<BTreeMap<String, Value>, String> {
        let mut out: BTreeMap<String, Value> =
            self.params.iter().map(|(k, d)| (k.clone(), d.default)).collect();
        for (key, &value) in overrides {
            out.insert(key.clone(), self.check(key, value)?);
        }
        Ok(out)
    }

    /// Maps unit-interval coordinates `u[key] ∈ [0, 1]` onto each parameter's prior:
    /// the inverse CDF of its uniform or log-uniform prior, rounded to the nearest
    /// integer for integer parameters. Calibration samplers (Latin hypercube, Sobol,
    /// ABC proposals) work in the unit cube and use this to get overrides. Only the
    /// requested keys are returned. Uses `oikos_math`, so the same `u` gives the same
    /// overrides on every host.
    pub fn sample(&self, u: &BTreeMap<String, f64>) -> Result<BTreeMap<String, Value>, String> {
        u.iter()
            .map(|(key, &x)| {
                let def = self.params.get(key).ok_or_else(|| format!("unknown parameter {key}"))?;
                let v = def.sample(x).map_err(|e| format!("parameter {key}: {e}"))?;
                Ok((key.clone(), v))
            })
            .collect()
    }

    /// Values as a JSON object (integers as JSON integers, reals as JSON floats).
    pub fn values_json(&self, values: &BTreeMap<String, Value>) -> serde_json::Value {
        let map = values
            .iter()
            .map(|(k, v)| {
                let n = match *v {
                    Value::Int(i) => serde_json::Value::from(i),
                    Value::Real(r) => serde_json::Value::from(r),
                };
                (k.clone(), n)
            })
            .collect();
        serde_json::Value::Object(map)
    }

    /// Whole registry as pretty JSON with sorted keys (for calibration tools).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("registry serializes")
    }
}

impl ParamDef {
    fn bounds(&self) -> (f64, f64) {
        match self.kind {
            Kind::Real { min, max } => (min, max),
            Kind::Integer { min, max } => (min as f64, max as f64),
            Kind::Probability => (0.0, 1.0),
        }
    }

    fn check(&self, value: Value) -> Result<Value, String> {
        let range = |lo: String, hi: String| format!("must be in [{lo}, {hi}] {}, got {value}", self.unit);
        match (&self.kind, value) {
            (Kind::Integer { min, max }, Value::Int(v)) => {
                if v < *min || v > *max {
                    return Err(range(min.to_string(), max.to_string()));
                }
                Ok(Value::Int(v))
            }
            (Kind::Integer { .. }, Value::Real(_)) => Err(format!("must be an integer, got {value}")),
            (_, v) => {
                let x = match v {
                    Value::Int(i) => i as f64,
                    Value::Real(r) => r,
                };
                let (lo, hi) = self.bounds();
                if !x.is_finite() || x < lo || x > hi {
                    return Err(range(lo.to_string(), hi.to_string()));
                }
                Ok(Value::Real(x))
            }
        }
    }

    fn sample(&self, u: f64) -> Result<Value, String> {
        if !(0.0..=1.0).contains(&u) {
            return Err(format!("unit-interval value must be in [0, 1], got {u}"));
        }
        let (lo, hi) = match self.prior {
            Prior::Fixed => return Err("prior is fixed (not calibrated), so it cannot be sampled".into()),
            Prior::Normal { .. } => return Err("sampling normal priors is not supported yet".into()),
            Prior::Uniform { lo, hi } | Prior::LogUniform { lo, hi } => (lo, hi),
        };
        // The endpoints map exactly onto the prior bounds; rounding could miss them.
        let x = match self.prior {
            _ if u == 0.0 => lo,
            _ if u == 1.0 => hi,
            Prior::LogUniform { .. } => lo * oikos_math::exp(u * oikos_math::ln(hi / lo)),
            _ => lo + u * (hi - lo),
        }
        .clamp(lo, hi);
        let value = match self.kind {
            // Round half up; floor is exact on every IEEE 754 target.
            Kind::Integer { .. } => Value::Int((x + 0.5).floor() as u64),
            _ => Value::Real(x),
        };
        self.check(value)
    }

    fn validate(&self) -> Result<(), String> {
        if self.unit.trim().is_empty() {
            return Err("unit must not be empty".into());
        }
        if self.description.trim().is_empty() {
            return Err("description must not be empty".into());
        }
        if self.group.trim().is_empty() {
            return Err("group must not be empty".into());
        }
        match self.kind {
            Kind::Real { min, max } if !(min.is_finite() && max.is_finite() && min <= max) => {
                return Err(format!("kind needs finite min <= max, got min {min}, max {max}"));
            }
            Kind::Integer { min, max } if min > max => {
                return Err(format!("kind needs min <= max, got min {min}, max {max}"));
            }
            _ => {}
        }
        self.check(self.default).map_err(|e| format!("default {e}"))?;
        let (lo, hi) = self.bounds();
        let inside = |a: f64, b: f64, what: &str| {
            if a.is_finite() && b.is_finite() && a < b && a >= lo && b <= hi {
                Ok(())
            } else {
                Err(format!("{what} prior needs {lo} <= lo < hi <= {hi}, got lo {a}, hi {b}"))
            }
        };
        match self.prior {
            Prior::Fixed => {}
            Prior::Uniform { lo: a, hi: b } => inside(a, b, "uniform")?,
            Prior::LogUniform { lo: a, hi: b } => {
                if a <= 0.0 {
                    return Err(format!("log_uniform prior needs lo > 0, got {a}"));
                }
                inside(a, b, "log_uniform")?;
            }
            Prior::Normal { mean, sd } => {
                if !(sd.is_finite() && sd > 0.0 && mean >= lo && mean <= hi) {
                    return Err(format!("normal prior needs sd > 0 and {lo} <= mean <= {hi}"));
                }
            }
        }
        match self.status {
            Status::Placeholder if self.note.as_deref().is_none_or(|n| n.trim().is_empty()) => {
                Err("placeholder parameters need a note saying why the value is used".into())
            }
            Status::Literature | Status::Calibrated if self.sources.is_empty() => {
                Err("literature and calibrated parameters need at least one source".into())
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests;
