use std::time::Duration;

use crate::dirty::DirtyRegion;
use crate::scrollback::ScrollbackSnapshot;

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

/// Deterministic native runtime counters for validation and performance probes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeRuntimePerfSnapshot {
    /// Number of non-empty PTY output batches pumped into terminal state.
    pub pty_output_batches: u64,
    /// Total PTY output bytes pumped into terminal state.
    pub pty_output_bytes: u64,
    /// Number of terminal-generated response writes sent back to the PTY.
    pub pty_response_writes: u64,
    /// Total terminal-generated response bytes sent back to the PTY.
    pub pty_response_bytes: u64,
    /// Number of app-originated PTY input writes.
    pub pty_input_writes: u64,
    /// Total app-originated PTY input bytes.
    pub pty_input_bytes: u64,
    /// Number of native key inputs encoded and written to the PTY.
    pub native_key_inputs: u64,
    /// Number of terminal mouse inputs encoded and written to the PTY.
    pub mouse_inputs: u64,
    /// Number of focus inputs encoded and written to the PTY.
    pub focus_inputs: u64,
    /// Number of clipboard paste actions that wrote text to the PTY.
    pub clipboard_pastes: u64,
    /// Total pasted text bytes written through the terminal paste path.
    pub paste_bytes: u64,
    /// Total committed text bytes written to the PTY.
    pub committed_text_bytes: u64,
    /// Number of successful terminal resize operations through the native runtime.
    pub resize_events: u64,
    /// Number of render attempts made by the native runtime.
    pub render_attempts: u64,
    /// Number of dirty terminal frames rendered through the renderer boundary.
    pub rendered_frames: u64,
    /// Total dirty regions consumed by successful render passes.
    pub rendered_dirty_regions: u64,
    /// Total dirty cells covered by successful render passes.
    pub rendered_dirty_cells: u64,
    /// Maximum dirty cells covered by one successful render pass.
    pub rendered_dirty_cells_max: u64,
    /// Number of render attempts skipped because no dirty regions were pending.
    pub clean_frame_skips: u64,
    /// Number of rendered frames with measured render duration samples.
    pub render_time_samples: u64,
    /// Total measured render-frame duration in nanoseconds.
    pub render_time_total_ns: u64,
    /// Average measured render-frame duration in nanoseconds.
    pub render_time_avg_ns: u64,
    /// Maximum measured render-frame duration in nanoseconds.
    pub render_time_max_ns: u64,
    /// Approximate p95 render-frame duration in nanoseconds, using fixed buckets.
    pub render_time_p95_ns: u64,
    /// Number of app-input-to-render latency samples.
    pub input_to_render_samples: u64,
    /// Total app-input-to-render latency in nanoseconds.
    pub input_to_render_total_ns: u64,
    /// Average app-input-to-render latency in nanoseconds.
    pub input_to_render_avg_ns: u64,
    /// Maximum app-input-to-render latency in nanoseconds.
    pub input_to_render_max_ns: u64,
    /// Approximate p95 app-input-to-render latency in nanoseconds, using fixed buckets.
    pub input_to_render_p95_ns: u64,
}

/// Deterministic runtime state footprint used by validation and memory-growth probes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeRuntimeStateSnapshot {
    /// Current visible terminal columns.
    pub terminal_cols: u16,
    /// Current visible terminal rows.
    pub terminal_rows: u16,
    /// Current visible terminal cell capacity.
    pub visible_cells: usize,
    /// Configured retained scrollback line limit.
    pub scrollback_limit: usize,
    /// Number of retained scrollback text rows.
    pub scrollback_lines: usize,
    /// Number of retained scrollback cell rows.
    pub scrollback_cell_rows: usize,
    /// Number of retained scrollback cells.
    pub scrollback_cells: usize,
    /// Maximum retained scrollback cells for the current column count and line cap.
    pub scrollback_cell_limit: usize,
}

/// Fixed-size duration histogram for bounded live-performance probes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct RuntimeDurationHistogram {
    buckets: [u64; RUNTIME_DURATION_BUCKETS_NS.len()],
}

impl RuntimeDurationHistogram {
    pub(super) fn record(&mut self, elapsed_ns: u64) {
        let bucket = RUNTIME_DURATION_BUCKETS_NS
            .iter()
            .position(|upper_bound| elapsed_ns <= *upper_bound)
            .unwrap_or(RUNTIME_DURATION_BUCKETS_NS.len() - 1);
        self.buckets[bucket] = self.buckets[bucket].saturating_add(1);
    }

    pub(super) fn p95_upper_bound_ns(self, samples: u64) -> u64 {
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

pub(super) fn saturating_duration_nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

pub(super) fn average_duration_nanos(total_ns: u64, samples: u64) -> u64 {
    if samples == 0 {
        return 0;
    }
    total_ns / samples
}

pub(super) fn add_usize_counter(counter: &mut u64, value: usize) {
    *counter = (*counter).saturating_add(saturating_usize_to_u64(value));
}

pub(super) fn scrollback_cell_count(scrollback: &ScrollbackSnapshot) -> usize {
    scrollback.cells.iter().map(Vec::len).sum()
}

pub(super) fn dirty_region_cell_count(regions: &[DirtyRegion]) -> u64 {
    regions.iter().fold(0_u64, |total, region| {
        total.saturating_add(u64::from(region.rows).saturating_mul(u64::from(region.cols)))
    })
}

pub(super) fn percentile_rank(samples: u64, percentile: u8) -> u64 {
    let samples = u128::from(samples);
    let percentile = u128::from(percentile);
    let rank = samples.saturating_mul(percentile).saturating_add(99) / 100;
    u64::try_from(rank).unwrap_or(u64::MAX)
}

fn saturating_usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::dirty::DirtyRegion;

    use super::*;

    #[test]
    fn runtime_perf_counter_adds_usize_values_with_saturation() {
        let mut counter = u64::MAX - 1;

        add_usize_counter(&mut counter, 8);

        assert_eq!(counter, u64::MAX);
    }

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
    fn runtime_dirty_region_cell_count_uses_widened_region_math() {
        let regions = [
            DirtyRegion {
                row: 0,
                col: 0,
                rows: u16::MAX,
                cols: u16::MAX,
            },
            DirtyRegion {
                row: 0,
                col: 0,
                rows: 2,
                cols: 3,
            },
        ];

        assert_eq!(
            dirty_region_cell_count(&regions),
            u64::from(u16::MAX) * u64::from(u16::MAX) + 6
        );
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
