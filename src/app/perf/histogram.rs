use std::time::Duration;

const RUNTIME_DURATION_BUCKETS_NS: [u64; 16] = [
    100_000,
    250_000,
    500_000,
    1_000_000,
    2_000_000,
    4_000_000,
    6_940_000,
    8_000_000,
    10_000_000,
    16_000_000,
    33_000_000,
    50_000_000,
    100_000_000,
    250_000_000,
    500_000_000,
    u64::MAX,
];

/// Fixed-size duration histogram for bounded live-performance probes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::app) struct RuntimeDurationHistogram {
    buckets: [u64; RUNTIME_DURATION_BUCKETS_NS.len()],
}

impl RuntimeDurationHistogram {
    pub(in crate::app) fn record(&mut self, elapsed_ns: u64) {
        let bucket = RUNTIME_DURATION_BUCKETS_NS
            .iter()
            .position(|upper_bound| elapsed_ns <= *upper_bound)
            .unwrap_or(RUNTIME_DURATION_BUCKETS_NS.len() - 1);
        self.buckets[bucket] = self.buckets[bucket].saturating_add(1);
    }

    pub(in crate::app) fn p95_upper_bound_ns(self, samples: u64) -> u64 {
        if samples == 0 {
            return 0;
        }
        let target_rank = percentile_rank(samples, 95);
        let mut cumulative = 0_u64;
        for (bucket, upper_bound) in self.buckets.iter().zip(RUNTIME_DURATION_BUCKETS_NS) {
            cumulative = cumulative.saturating_add(*bucket);
            if cumulative >= target_rank {
                return upper_bound;
            }
        }
        u64::MAX
    }
}

pub(in crate::app) fn saturating_duration_nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

pub(in crate::app) fn average_duration_nanos(total_ns: u64, samples: u64) -> u64 {
    if samples == 0 {
        return 0;
    }
    total_ns / samples
}

pub(in crate::app) fn percentile_rank(samples: u64, percentile: u8) -> u64 {
    let samples = u128::from(samples);
    let percentile = u128::from(percentile);
    let rank = samples.saturating_mul(percentile).saturating_add(99) / 100;
    u64::try_from(rank).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn runtime_perf_duration_nanos_reports_u64_values() {
        let duration = Duration::from_nanos(42);

        assert_eq!(saturating_duration_nanos(duration), 42);
    }

    #[test]
    fn runtime_perf_average_duration_reports_zero_without_samples() {
        assert_eq!(average_duration_nanos(42, 0), 0);
        assert_eq!(average_duration_nanos(42, 3), 14);
    }

    #[test]
    fn runtime_duration_histogram_reports_bucketed_p95_upper_bound() {
        let mut histogram = RuntimeDurationHistogram::default();
        for elapsed_ns in [
            50_000_u64, 120_000, 300_000, 900_000, 1_500_000, 3_000_000, 6_500_000, 7_500_000,
            9_500_000, 15_000_000,
        ] {
            histogram.record(elapsed_ns);
        }

        assert_eq!(histogram.p95_upper_bound_ns(10), 16_000_000);
    }

    #[test]
    fn runtime_duration_histogram_reports_zero_without_samples() {
        let histogram = RuntimeDurationHistogram::default();

        assert_eq!(histogram.p95_upper_bound_ns(0), 0);
    }

    #[test]
    fn percentile_rank_rounds_up() {
        assert_eq!(percentile_rank(1, 95), 1);
        assert_eq!(percentile_rank(20, 95), 19);
        assert_eq!(percentile_rank(21, 95), 20);
    }
}
