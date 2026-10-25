//! Simulation time.
//!
//! The engine counts integer ticks from the epoch (12,000 BCE). The base unit is one
//! day; mechanisms declare their own period in ticks.

/// Ticks per simulated year (365-day years; calendars are a presentation concern).
pub const TICKS_PER_YEAR: u64 = 365;

/// Calendar year of the epoch (negative = BCE, astronomical numbering not applied).
pub const EPOCH_YEAR: i64 = -12_000;

/// A point in simulated time, in days since the epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Tick(pub u64);

impl Tick {
    pub const EPOCH: Tick = Tick(0);

    pub const fn from_years(years: u64) -> Tick {
        Tick(years * TICKS_PER_YEAR)
    }

    /// Whole simulated years since the epoch.
    pub const fn years(self) -> u64 {
        self.0 / TICKS_PER_YEAR
    }

    /// Calendar year (BCE negative) for display.
    pub const fn calendar_year(self) -> i64 {
        EPOCH_YEAR + self.years() as i64
    }

    pub const fn advance(self, ticks: u64) -> Tick {
        Tick(self.0 + ticks)
    }

    /// True if a mechanism with `period` ticks fires at this tick.
    pub const fn fires(self, period: u64) -> bool {
        self.0 % period == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar() {
        assert_eq!(Tick::EPOCH.calendar_year(), -12_000);
        assert_eq!(Tick::from_years(9_000).calendar_year(), -3_000);
        assert!(Tick(730).fires(365));
        assert!(!Tick(731).fires(365));
    }
}
