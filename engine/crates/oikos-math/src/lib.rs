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

/// The standard normal distribution function, `erfc(-x / sqrt 2) / 2` (no cancellation
/// in the lower tail).
pub fn normal_cdf(x: f64) -> f64 {
    0.5 * libm::erfc(-x * core::f64::consts::FRAC_1_SQRT_2)
}

/// The inverse of [`normal_cdf`]: Acklam's rational approximation (relative error below
/// 1.2e-9), refined by one Halley step on `normal_cdf`, to near full precision. Above
/// one half it is the mirror of the lower half (`1 - p` is exact there), so it is odd
/// about one half exactly. 0 and 1 give minus and plus infinity.
pub fn normal_quantile(p: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    if p > 0.5 {
        return -normal_quantile(1.0 - p);
    }
    const A: [f64; 6] = [
        -3.969683028665376e1,
        2.209460984245205e2,
        -2.759285104469687e2,
        1.38357751867269e2,
        -3.066479806614716e1,
        2.506628277459239,
    ];
    const B: [f64; 5] = [
        -5.447609879822406e1,
        1.615858368580409e2,
        -1.556989798598866e2,
        6.680131188771972e1,
        -1.328068155288572e1,
    ];
    const C: [f64; 6] = [
        -7.784894002430293e-3,
        -3.223964580411365e-1,
        -2.400758277161838,
        -2.549732539343734,
        4.374664141464968,
        2.938163982698783,
    ];
    const D: [f64; 4] = [7.784695709041462e-3, 3.224671290700398e-1, 2.445134137142996, 3.754408661907416];
    let x = if p < 0.02425 {
        let q = sqrt(-2.0 * ln(p));
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    };
    // Halley's step: e / density, with the second-order correction.
    let e = normal_cdf(x) - p;
    let u = e * sqrt(2.0 * core::f64::consts::PI) * exp(0.5 * x * x);
    x - u / (1.0 + 0.5 * x * u)
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

    /// Fixed bit patterns of the normal functions, checked on every target.
    #[test]
    fn normal_golden_bits() {
        let samples = [
            normal_cdf(-1.3),
            normal_cdf(0.7),
            normal_quantile(0.01),
            normal_quantile(0.3),
            normal_quantile(0.9),
        ];
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for s in samples {
            for b in s.to_bits().to_le_bytes() {
                h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
            }
        }
        assert_eq!(h, NORMAL_GOLDEN, "{h}");
    }

    const NORMAL_GOLDEN: u64 = 14_008_544_490_679_630_273;

    /// Known answers (R's `pnorm` and `qnorm`), to near the last bit.
    #[test]
    fn the_normal_distribution_function_and_its_inverse() {
        let close = |a: f64, b: f64, tol: f64| libm::fabs(a - b) <= tol * libm::fabs(b).max(1e-300);
        assert_eq!(normal_cdf(0.0), 0.5);
        assert!(close(normal_cdf(1.959963984540054), 0.975, 1e-15));
        assert!(close(normal_cdf(-1.0), 0.15865525393145707, 1e-15));
        assert!(
            close(normal_cdf(-8.0), 6.220960574271785e-16, 1e-13),
            "the far tail keeps its relative accuracy"
        );
        assert_eq!(normal_quantile(0.5), 0.0);
        assert!(close(normal_quantile(0.975), 1.959963984540054, 1e-15));
        assert!(close(normal_quantile(0.15865525393145707), -1.0, 1e-14));
        assert!(close(normal_quantile(0.01), -2.3263478740408408, 1e-15));
        assert!(close(normal_quantile(1e-10), -6.361340902404056, 1e-14));
        assert!(close(normal_quantile(0.999), 3.090232306167813, 1e-14));
        assert_eq!(normal_quantile(0.0), f64::NEG_INFINITY);
        assert_eq!(normal_quantile(1.0), f64::INFINITY);
        for i in 1..1000 {
            let p = i as f64 / 1000.0;
            assert!(close(normal_cdf(normal_quantile(p)), p, 1e-14), "round trip at {p}");
            assert!(normal_quantile(p) > normal_quantile(p - 0.001), "increasing at {p}");
        }
    }

    #[test]
    fn close_to_std_reference() {
        assert!(libm::fabs(exp(1.0) - core::f64::consts::E) < 1e-15);
        assert!(libm::fabs(ln(core::f64::consts::E) - 1.0) < 1e-15);
    }
}
