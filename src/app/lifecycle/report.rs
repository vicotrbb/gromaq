//! Native app run reporting and presented-frame interval accounting.

use std::time::Instant;

use super::super::perf::{
    RuntimeDurationHistogram, average_duration_nanos, saturating_duration_nanos,
};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Native app event-loop report captured after the app exits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeAppRunReport {
    /// Count of native windows created during the run.
    pub windows_created: u64,
    /// Count of native redraw requests scheduled by app logic.
    pub redraw_requests: u64,
    /// Count of redraw events observed by the app boundary.
    pub frames_presented: u64,
    /// Active monitor refresh rate in millihertz, if the platform reported one.
    pub monitor_refresh_millihertz: Option<u32>,
    /// Effective FPS target used for presented-frame interval accounting.
    pub frame_interval_target_fps: u32,
    /// Count of measured intervals between presented frames.
    pub frame_interval_samples: u64,
    /// Total measured presented-frame interval duration in nanoseconds.
    pub frame_interval_total_ns: u64,
    /// Average measured presented-frame interval duration in nanoseconds.
    pub frame_interval_avg_ns: u64,
    /// Maximum measured presented-frame interval duration in nanoseconds.
    pub frame_interval_max_ns: u64,
    /// Approximate p95 presented-frame interval in nanoseconds, using fixed buckets.
    pub frame_interval_p95_ns: u64,
    /// Count of measured intervals that exceeded the effective target frame interval.
    pub frame_intervals_over_target: u64,
    /// Count of measured intervals that exceeded twice the effective target frame interval.
    pub frame_intervals_over_double_target: u64,
    /// Number of target frame intervals missed between presented frames.
    pub dropped_frames: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct PresentedFrameIntervals {
    last_presented_at: Option<Instant>,
    samples: u64,
    total_ns: u64,
    avg_ns: u64,
    max_ns: u64,
    intervals_over_target: u64,
    intervals_over_double_target: u64,
    dropped_frames: u64,
    histogram: RuntimeDurationHistogram,
}

impl PresentedFrameIntervals {
    pub(super) fn record_presented_at(&mut self, presented_at: Instant, target_fps: u32) {
        if let Some(last_presented_at) = self.last_presented_at {
            let elapsed_ns = saturating_duration_nanos(
                presented_at.saturating_duration_since(last_presented_at),
            );
            let target_interval_ns = target_interval_nanos(target_fps);
            if elapsed_ns > target_interval_ns {
                self.intervals_over_target = self.intervals_over_target.saturating_add(1);
            }
            if elapsed_ns > target_interval_ns.saturating_mul(2) {
                self.intervals_over_double_target =
                    self.intervals_over_double_target.saturating_add(1);
            }
            self.dropped_frames = self
                .dropped_frames
                .saturating_add(dropped_frames_for_interval(elapsed_ns, target_fps));
            self.samples = self.samples.saturating_add(1);
            self.total_ns = self.total_ns.saturating_add(elapsed_ns);
            self.avg_ns = average_duration_nanos(self.total_ns, self.samples);
            self.max_ns = self.max_ns.max(elapsed_ns);
            self.histogram.record(elapsed_ns);
        }
        self.last_presented_at = Some(presented_at);
    }

    pub(super) fn run_report(
        self,
        windows_created: u64,
        redraw_requests: u64,
        frames_presented: u64,
        monitor_refresh_millihertz: Option<u32>,
        frame_interval_target_fps: u32,
    ) -> NativeAppRunReport {
        NativeAppRunReport {
            windows_created,
            redraw_requests,
            frames_presented,
            monitor_refresh_millihertz,
            frame_interval_target_fps,
            frame_interval_samples: self.samples,
            frame_interval_total_ns: self.total_ns,
            frame_interval_avg_ns: self.avg_ns,
            frame_interval_max_ns: self.max_ns,
            frame_interval_p95_ns: self.histogram.p95_upper_bound_ns(self.samples),
            frame_intervals_over_target: self.intervals_over_target,
            frame_intervals_over_double_target: self.intervals_over_double_target,
            dropped_frames: self.dropped_frames,
        }
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

        intervals.record_presented_at(started_at, 144);
        intervals.record_presented_at(started_at + std::time::Duration::from_nanos(6_944_444), 144);
        intervals.record_presented_at(
            started_at + std::time::Duration::from_nanos(27_777_776),
            144,
        );

        let report = intervals.run_report(1, 2, 3, None, 144);

        assert_eq!(report.frame_interval_samples, 2);
        assert_eq!(report.monitor_refresh_millihertz, None);
        assert_eq!(report.frame_interval_target_fps, 144);
        assert_eq!(report.frame_intervals_over_target, 1);
        assert_eq!(report.frame_intervals_over_double_target, 1);
        assert_eq!(report.dropped_frames, 2);
    }
}
