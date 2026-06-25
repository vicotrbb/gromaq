//! Presented-frame interval accounting for native app run reports.

use std::time::Instant;

use super::{NativeAppRunReport, NativeAppRunReportInput};
use crate::app::perf::{
    RuntimeDurationHistogram, average_duration_nanos, percentile_rank, saturating_duration_nanos,
};

const NANOS_PER_SECOND: u64 = 1_000_000_000;
const PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app::lifecycle) struct PresentedFrameIntervals {
    last_presented_at: Option<Instant>,
    samples: u64,
    total_ns: u64,
    avg_ns: u64,
    max_ns: u64,
    max_sample_index: u64,
    intervals_over_target: u64,
    intervals_over_double_target: u64,
    dropped_frames: u64,
    first_dropped_frame_interval_sample: u64,
    last_dropped_frame_interval_sample: u64,
    histogram: RuntimeDurationHistogram,
    interval_samples_ns: [u64; PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY],
    interval_sample_len: usize,
}

impl Default for PresentedFrameIntervals {
    fn default() -> Self {
        Self {
            last_presented_at: None,
            samples: 0,
            total_ns: 0,
            avg_ns: 0,
            max_ns: 0,
            max_sample_index: 0,
            intervals_over_target: 0,
            intervals_over_double_target: 0,
            dropped_frames: 0,
            first_dropped_frame_interval_sample: 0,
            last_dropped_frame_interval_sample: 0,
            histogram: RuntimeDurationHistogram::default(),
            interval_samples_ns: [0; PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY],
            interval_sample_len: 0,
        }
    }
}

impl PresentedFrameIntervals {
    pub(in crate::app::lifecycle) fn record_presented_at(
        &mut self,
        presented_at: Instant,
        target_fps: u32,
        presented_frame_index: u64,
        warmup_frames: u64,
    ) {
        if let Some(last_presented_at) = self.last_presented_at {
            let elapsed_ns = saturating_duration_nanos(
                presented_at.saturating_duration_since(last_presented_at),
            );
            if presented_frame_index <= warmup_frames {
                self.last_presented_at = Some(presented_at);
                return;
            }
            let target_interval_ns = target_interval_nanos(target_fps);
            let sample_index = self.samples.saturating_add(1);
            if elapsed_ns > target_interval_ns {
                self.intervals_over_target = self.intervals_over_target.saturating_add(1);
            }
            if elapsed_ns > target_interval_ns.saturating_mul(2) {
                self.intervals_over_double_target =
                    self.intervals_over_double_target.saturating_add(1);
            }
            let dropped_frames = dropped_frames_for_interval(elapsed_ns, target_fps);
            if dropped_frames > 0 {
                if self.first_dropped_frame_interval_sample == 0 {
                    self.first_dropped_frame_interval_sample = sample_index;
                }
                self.last_dropped_frame_interval_sample = sample_index;
            }
            self.dropped_frames = self.dropped_frames.saturating_add(dropped_frames);
            self.samples = sample_index;
            self.total_ns = self.total_ns.saturating_add(elapsed_ns);
            self.avg_ns = average_duration_nanos(self.total_ns, self.samples);
            if elapsed_ns > self.max_ns {
                self.max_ns = elapsed_ns;
                self.max_sample_index = sample_index;
            }
            self.histogram.record(elapsed_ns);
            if self.interval_sample_len < PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY {
                self.interval_samples_ns[self.interval_sample_len] = elapsed_ns;
                self.interval_sample_len += 1;
            }
        }
        self.last_presented_at = Some(presented_at);
    }

    pub(in crate::app::lifecycle) fn run_report(
        self,
        input: NativeAppRunReportInput,
    ) -> NativeAppRunReport {
        NativeAppRunReport {
            windows_created: input.windows_created,
            redraw_requests: input.redraw_requests,
            redraw_attempts: input.redraw_attempts,
            frames_presented: input.frames_presented,
            surface_frame_timeouts: input.surface_frame_timeouts,
            surface_frame_occluded: input.surface_frame_occluded,
            monitor_refresh_millihertz: input.monitor_refresh_millihertz,
            surface_present_mode: input.surface_present_mode,
            window_width_px: input.window_width_px,
            window_height_px: input.window_height_px,
            window_scale_milliscale: input.window_scale_milliscale,
            glyph_frame_presented: input.glyph_frame_presented,
            glyph_frame_width: input.glyph_frame_width,
            glyph_frame_height: input.glyph_frame_height,
            glyph_frame_glyph_quads: input.glyph_frame_glyph_quads,
            glyph_frame_background_quads: input.glyph_frame_background_quads,
            glyph_frame_decoration_quads: input.glyph_frame_decoration_quads,
            glyph_frame_cursor_quads: input.glyph_frame_cursor_quads,
            glyph_frame_atlas_bytes: input.glyph_frame_atlas_bytes,
            glyph_frame_atlas_occupied_slots: input.glyph_frame_atlas_occupied_slots,
            glyph_frame_snapshot_written: input.glyph_frame_snapshot_written,
            glyph_frame_snapshot_bytes: input.glyph_frame_snapshot_bytes,
            glyph_frame_snapshot_width: input.glyph_frame_snapshot_width,
            glyph_frame_snapshot_height: input.glyph_frame_snapshot_height,
            frame_interval_target_fps: input.frame_interval_target_fps,
            frame_interval_warmup_frames: input.frame_interval_warmup_frames,
            frame_interval_samples: self.samples,
            frame_interval_total_ns: self.total_ns,
            frame_interval_avg_ns: self.avg_ns,
            frame_interval_max_ns: self.max_ns,
            frame_interval_max_sample_index: self.max_sample_index,
            frame_interval_p95_ns: self.histogram.p95_upper_bound_ns(self.samples),
            frame_interval_p95_exact_ns: self.exact_p95_ns(),
            frame_intervals_over_target: self.intervals_over_target,
            frame_intervals_over_double_target: self.intervals_over_double_target,
            dropped_frames: self.dropped_frames,
            first_dropped_frame_interval_sample: self.first_dropped_frame_interval_sample,
            last_dropped_frame_interval_sample: self.last_dropped_frame_interval_sample,
        }
    }

    fn exact_p95_ns(self) -> u64 {
        if self.samples == 0 || usize::try_from(self.samples).ok() != Some(self.interval_sample_len)
        {
            return 0;
        }
        let mut samples = self.interval_samples_ns;
        samples[..self.interval_sample_len].sort_unstable();
        let rank = percentile_rank(self.samples, 95).saturating_sub(1);
        let index = usize::try_from(rank).unwrap_or(self.interval_sample_len - 1);
        samples[index]
    }
}

fn dropped_frames_for_interval(elapsed_ns: u64, target_fps: u32) -> u64 {
    let target_interval_ns = target_interval_nanos(target_fps);
    let intervals = elapsed_ns / target_interval_ns;
    intervals.saturating_sub(1)
}

fn target_interval_nanos(target_fps: u32) -> u64 {
    NANOS_PER_SECOND / u64::from(target_fps.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropped_frames_for_interval_counts_missed_target_periods() {
        assert_eq!(dropped_frames_for_interval(6_944_444, 144), 0);
        assert_eq!(dropped_frames_for_interval(13_888_888, 144), 1);
        assert_eq!(dropped_frames_for_interval(27_777_776, 144), 3);
    }

    #[test]
    fn presented_frame_intervals_report_dropped_frames() {
        let started_at = Instant::now();
        let mut intervals = PresentedFrameIntervals::default();

        intervals.record_presented_at(started_at, 144, 1, 0);
        intervals.record_presented_at(
            started_at + std::time::Duration::from_nanos(6_944_444),
            144,
            2,
            0,
        );
        intervals.record_presented_at(
            started_at + std::time::Duration::from_nanos(27_777_776),
            144,
            3,
            0,
        );

        let report = intervals.run_report(NativeAppRunReportInput {
            windows_created: 1,
            redraw_requests: 2,
            frames_presented: 3,
            frame_interval_target_fps: 144,
            ..NativeAppRunReportInput::default()
        });

        assert_eq!(report.frame_interval_samples, 2);
        assert_eq!(report.monitor_refresh_millihertz, None);
        assert_eq!(report.surface_present_mode, None);
        assert_eq!(report.window_width_px, None);
        assert_eq!(report.window_height_px, None);
        assert_eq!(report.window_scale_milliscale, None);
        assert_eq!(report.frame_interval_target_fps, 144);
        assert_eq!(report.frame_interval_warmup_frames, 0);
        assert_eq!(report.frame_interval_max_sample_index, 2);
        assert_eq!(report.frame_interval_p95_exact_ns, 20_833_332);
        assert_eq!(report.frame_intervals_over_target, 1);
        assert_eq!(report.frame_intervals_over_double_target, 1);
        assert_eq!(report.dropped_frames, 2);
        assert_eq!(report.first_dropped_frame_interval_sample, 2);
        assert_eq!(report.last_dropped_frame_interval_sample, 2);
    }

    #[test]
    fn presented_frame_intervals_exact_p95_requires_full_bounded_sample_set() {
        let started_at = Instant::now();
        let mut intervals = PresentedFrameIntervals::default();

        for frame in 0..=PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY {
            intervals.record_presented_at(
                started_at + std::time::Duration::from_nanos(frame as u64),
                144,
                u64::try_from(frame + 1).unwrap(),
                0,
            );
        }

        let report = intervals.run_report(NativeAppRunReportInput {
            frames_presented: u64::try_from(PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY + 1).unwrap(),
            frame_interval_target_fps: 144,
            ..NativeAppRunReportInput::default()
        });

        assert_eq!(
            report.frame_interval_samples,
            u64::try_from(PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY).unwrap()
        );
        assert_eq!(report.frame_interval_p95_exact_ns, 1);

        intervals.record_presented_at(
            started_at
                + std::time::Duration::from_nanos(
                    u64::try_from(PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY + 1).unwrap(),
                ),
            144,
            u64::try_from(PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY + 2).unwrap(),
            0,
        );
        let report = intervals.run_report(NativeAppRunReportInput {
            frames_presented: u64::try_from(PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY + 2).unwrap(),
            frame_interval_target_fps: 144,
            ..NativeAppRunReportInput::default()
        });

        assert_eq!(
            report.frame_interval_samples,
            u64::try_from(PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY + 1).unwrap()
        );
        assert_eq!(report.frame_interval_p95_exact_ns, 0);
    }
}
