//! Lightweight measurements for the Phase 0 rendering harness.

use std::time::Duration;

/// Insert-latency counters for model updates.
///
/// These measure the time spent mutating the Slint model on the UI thread, not
/// frame time. They are intentionally cheap enough to run in every batch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InsertMetrics {
    last_micros: u64,
    worst_micros: u64,
}

impl InsertMetrics {
    pub fn record(&mut self, elapsed: Duration) {
        let micros = u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX);
        self.last_micros = micros;
        self.worst_micros = self.worst_micros.max(micros);
    }

    pub const fn last_micros(self) -> u64 {
        self.last_micros
    }

    pub const fn worst_micros(self) -> u64 {
        self.worst_micros
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_last_and_worst_values() {
        let mut metrics = InsertMetrics::default();
        metrics.record(Duration::from_micros(250));
        metrics.record(Duration::from_micros(900));
        metrics.record(Duration::from_micros(400));

        assert_eq!(metrics.last_micros(), 400);
        assert_eq!(metrics.worst_micros(), 900);
    }

    #[test]
    fn reset_clears_both_counters() {
        let mut metrics = InsertMetrics::default();
        metrics.record(Duration::from_millis(1));
        metrics.reset();

        assert_eq!(metrics, InsertMetrics::default());
    }
}
