//! Stable entity identifiers.
//!
//! An id encodes the grid cell where the entity was born and a per-cell birth
//! counter. Both are owned by whichever partition owns the cell, so ids never depend
//! on how the world is partitioned or on thread scheduling.

/// Bits reserved for the per-cell birth counter (~1.1e12 births per cell).
const COUNTER_BITS: u32 = 40;
const COUNTER_MASK: u64 = (1 << COUNTER_BITS) - 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u64);

impl EntityId {
    /// Builds an id from a birth cell (grid-local index, < 2^24) and its counter.
    pub const fn new(birth_cell: u32, counter: u64) -> EntityId {
        assert!(birth_cell < (1 << (64 - COUNTER_BITS)));
        assert!(counter <= COUNTER_MASK);
        EntityId(((birth_cell as u64) << COUNTER_BITS) | counter)
    }

    pub const fn birth_cell(self) -> u32 {
        (self.0 >> COUNTER_BITS) as u32
    }

    pub const fn counter(self) -> u64 {
        self.0 & COUNTER_MASK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let id = EntityId::new(2400, 123_456);
        assert_eq!(id.birth_cell(), 2400);
        assert_eq!(id.counter(), 123_456);
    }
}
