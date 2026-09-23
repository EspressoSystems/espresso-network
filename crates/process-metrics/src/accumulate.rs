//! Turning absolute readings into counter deltas.
//!
//! The kernel and the tokio runtime both report monotonic totals, while a
//! `Counter` only takes increments, so each sampled total needs the previous
//! reading kept across ticks.

/// Accumulates fractional units (µs, ns or ticks) into a counter measured in whole seconds,
/// preserving sub-second precision across many ticks.
#[derive(Default)]
pub(crate) struct SecondsAccumulator {
    /// Last absolute reading from the source, in its own unit (µs, ns or ticks).
    last: Option<u64>,
    /// Sub-second remainder carried between calls, in the source unit.
    remainder: u64,
}

impl SecondsAccumulator {
    /// Feed an absolute monotonic reading. Returns whole-seconds delta to add to the counter.
    pub(crate) fn observe(&mut self, current: u64, units_per_second: u64) -> usize {
        let Some(prev) = self.last.replace(current) else {
            return 0;
        };
        let delta = current.saturating_sub(prev);
        let total = self.remainder + delta;
        let whole = total / units_per_second;
        self.remainder = total % units_per_second;
        whole as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_accumulator_first_sample_is_zero() {
        let mut acc = SecondsAccumulator::default();
        assert_eq!(acc.observe(123_456, 1_000_000), 0);
    }

    #[test]
    fn seconds_accumulator_preserves_remainder() {
        let mut acc = SecondsAccumulator::default();
        // First call seeds the baseline.
        assert_eq!(acc.observe(0, 1_000_000), 0);
        // 0.5s delta — no whole second yet.
        assert_eq!(acc.observe(500_000, 1_000_000), 0);
        // Another 0.6s delta — one whole second, 0.1s remainder.
        assert_eq!(acc.observe(1_100_000, 1_000_000), 1);
        // Another 0.95s — total now 1.05s of remainder + delta → 1 sec.
        assert_eq!(acc.observe(2_050_000, 1_000_000), 1);
    }

    #[test]
    fn seconds_accumulator_handles_counter_reset() {
        let mut acc = SecondsAccumulator::default();
        acc.observe(10_000_000, 1_000_000);
        // Apparent regression (e.g. proc remount or wraparound), saturate to 0.
        assert_eq!(acc.observe(5_000_000, 1_000_000), 0);
        // After saturating, `last` should equal the most recent reading; the next
        // legitimate delta from there should still register.
        assert_eq!(acc.observe(6_000_000, 1_000_000), 1);
    }
}
