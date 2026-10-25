//! Probabilities as integer thresholds.
//!
//! A probability computed in floating point is converted once to a threshold in
//! `0..=2^32`, then compared against a uniform `u32` draw. Every stochastic decision
//! is an exact integer comparison on every platform.

const SCALE: f64 = 4_294_967_296.0; // 2^32

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Probability(u64);

impl Probability {
    pub const NEVER: Probability = Probability(0);
    pub const ALWAYS: Probability = Probability(1 << 32);

    /// Converts `p` (clamped to `[0, 1]`). Panics on NaN: NaN is always a bug.
    pub fn from_f64(p: f64) -> Probability {
        assert!(!p.is_nan(), "probability is NaN");
        if p <= 0.0 {
            Probability::NEVER
        } else if p >= 1.0 {
            Probability::ALWAYS
        } else {
            // Multiplication by a power of two and truncation are exact.
            Probability((p * SCALE) as u64)
        }
    }

    pub const fn threshold(self) -> u64 {
        self.0
    }

    /// True with this probability, given a uniform 32-bit draw.
    pub const fn hit(self, draw: u32) -> bool {
        (draw as u64) < self.0
    }
}

/// Rounds a non-negative real to an integer, up with probability equal to its
/// fractional part. Unbiased and exact given the draw.
pub fn stochastic_round(x: f64, draw: u32) -> u64 {
    assert!(!x.is_nan() && x >= 0.0, "stochastic_round of {x}");
    let whole = x as u64; // truncation toward zero, saturating
    let frac = x - whole as f64;
    whole + Probability::from_f64(frac).hit(draw) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extremes() {
        assert!(!Probability::NEVER.hit(0));
        assert!(Probability::ALWAYS.hit(u32::MAX));
        assert!(Probability::from_f64(0.5).hit(0x7fff_ffff));
        assert!(!Probability::from_f64(0.5).hit(0x8000_0000));
    }

    #[test]
    fn rounding() {
        assert_eq!(stochastic_round(3.0, u32::MAX), 3);
        assert_eq!(stochastic_round(3.25, 0), 4);
        assert_eq!(stochastic_round(3.25, u32::MAX), 3);
    }
}
