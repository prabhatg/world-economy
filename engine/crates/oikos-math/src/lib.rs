//! Bit-reproducible math for the Oikos engine.
//!
//! IEEE 754 guarantees correctly rounded `+ - * / sqrt` on every platform we target,
//! but platform `libm`s differ for transcendental functions. All mechanism code must
//! call these functions instead of the `f64` methods (`x.exp()`, `x.ln()`, ...).
//!
//! The implementations come from the pure-Rust `libm` crate (a port of musl), pinned
//! to an exact version: changing that version is a reproducibility-breaking change and
//! must update the golden hashes.

#![no_std]

#[inline]
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}

#[inline]
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}

#[inline]
pub fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}

#[inline]
pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

#[inline]
pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

#[inline]
pub fn tanh(x: f64) -> f64 {
    libm::tanh(x)
}

/// Logistic function `1 / (1 + e^-x)`.
#[inline]
pub fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + exp(-x))
}

/// Correctly rounded per IEEE 754 on all targets.
#[inline]
pub fn sqrt(x: f64) -> f64 {
    libm::sqrt(x)
}

/// Fails loudly on NaN or infinity; use at mechanism boundaries.
#[inline]
#[track_caller]
pub fn finite(x: f64) -> f64 {
    assert!(x.is_finite(), "non-finite value {x}");
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixed bit patterns: if these change, results are no longer reproducible.
    #[test]
    fn golden_bits() {
        let samples = [exp(1.0), ln(10.0), pow(1.5, 2.25), sin(0.5), cos(0.5), tanh(0.3), logistic(0.7)];
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for s in samples {
            for b in s.to_bits().to_le_bytes() {
                h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
            }
        }
        assert_eq!(h, GOLDEN);
    }

    const GOLDEN: u64 = 1_898_737_581_958_300_328;

    #[test]
    fn close_to_std_reference() {
        assert!(libm::fabs(exp(1.0) - core::f64::consts::E) < 1e-15);
        assert!(libm::fabs(ln(core::f64::consts::E) - 1.0) < 1e-15);
    }
}
