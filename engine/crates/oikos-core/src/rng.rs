//! Counter-based random numbers (Philox4x32-10, Salmon et al. 2011).
//!
//! There is no RNG state to carry or split: every draw is a pure function of
//! `(run seed, stream, entity, tick, index)`. Results are therefore independent of
//! partitioning, thread count and evaluation order.

const M0: u32 = 0xD251_1F53;
const M1: u32 = 0xCD9E_8D57;
const W0: u32 = 0x9E37_79B9;
const W1: u32 = 0xBB67_AE85;

/// Raw Philox4x32 with 10 rounds.
pub fn philox4x32_10(mut ctr: [u32; 4], mut key: [u32; 2]) -> [u32; 4] {
    for round in 0..10 {
        if round > 0 {
            key[0] = key[0].wrapping_add(W0);
            key[1] = key[1].wrapping_add(W1);
        }
        let p0 = (M0 as u64) * (ctr[0] as u64);
        let p1 = (M1 as u64) * (ctr[2] as u64);
        ctr = [
            ((p1 >> 32) as u32) ^ ctr[1] ^ key[0],
            p1 as u32,
            ((p0 >> 32) as u32) ^ ctr[3] ^ key[1],
            p0 as u32,
        ];
    }
    ctr
}

/// Independent random streams, one per mechanism or purpose. Values are part of the
/// reproducibility contract: never renumber an existing stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Stream {
    Environment = 1,
    Initialization = 2,
    Growth = 3,
    Fission = 4,
    Migration = 5,
}

/// A keyed generator for one run and stream.
#[derive(Clone, Copy, Debug)]
pub struct Philox4x32 {
    key: [u32; 2],
}

impl Philox4x32 {
    pub fn new(run_seed: u64, stream: Stream) -> Philox4x32 {
        // Mix the stream into the seed so streams are unrelated even for seed 0.
        let k0 = (run_seed as u32) ^ (stream as u32).wrapping_mul(0x85EB_CA6B);
        let k1 = ((run_seed >> 32) as u32) ^ (stream as u32).wrapping_mul(0xC2B2_AE35);
        Philox4x32 { key: [k0, k1] }
    }

    /// Four 32-bit words for `(subject, tick, index)`.
    pub fn block(&self, subject: u64, tick: u64, index: u32) -> [u32; 4] {
        let ctr = [
            subject as u32,
            (subject >> 32) as u32,
            tick as u32,
            index ^ (((tick >> 32) as u32) << 16),
        ];
        philox4x32_10(ctr, self.key)
    }

    /// One 32-bit word; `lane` selects one of the four words of the block.
    pub fn u32(&self, subject: u64, tick: u64, index: u32, lane: usize) -> u32 {
        self.block(subject, tick, index)[lane & 3]
    }

    /// Uniform integer in `0..n` (n > 0) via Lemire's multiply-shift; bias < n / 2^32,
    /// acceptable for simulation use and fully deterministic.
    pub fn below(&self, subject: u64, tick: u64, index: u32, lane: usize, n: u32) -> u32 {
        assert!(n > 0);
        (((self.u32(subject, tick, index, lane) as u64) * (n as u64)) >> 32) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Known-answer vectors from the Random123 distribution (kat_vectors).
    #[test]
    fn known_answers() {
        assert_eq!(
            philox4x32_10([0; 4], [0; 2]),
            [0x6627_e8d5, 0xe169_c58d, 0xbc57_ac4c, 0x9b00_dbd8]
        );
        assert_eq!(
            philox4x32_10([u32::MAX; 4], [u32::MAX; 2]),
            [0x408f_276d, 0x41c8_3b0e, 0xa20b_c7c6, 0x6d54_51fd]
        );
        assert_eq!(
            philox4x32_10(
                [0x243f_6a88, 0x85a3_08d3, 0x1319_8a2e, 0x0370_7344],
                [0xa409_3822, 0x299f_31d0]
            ),
            [0xd16c_fe09, 0x94fd_cceb, 0x5001_e420, 0x2412_6ea1]
        );
    }

    #[test]
    fn streams_differ() {
        let a = Philox4x32::new(7, Stream::Growth).u32(1, 2, 3, 0);
        let b = Philox4x32::new(7, Stream::Migration).u32(1, 2, 3, 0);
        assert_ne!(a, b);
    }

    #[test]
    fn below_in_range() {
        let g = Philox4x32::new(1, Stream::Fission);
        for i in 0..1000 {
            assert!(g.below(i, 0, 0, 0, 6) < 6);
        }
    }
}
