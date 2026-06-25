//! Presented-frame interval accounting for native app run reports.

mod run_report;

use std::time::Instant;

#[cfg(test)]
use super::NativeAppRunReportInput;
use crate::app::perf::{
    RuntimeDurationHistogram, average_duration_nanos, saturating_duration_nanos,
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

    pub(in crate::app::lifecycle) fn estimated_fps(&self, fallback_fps: u32) -> u32 {
        if self.avg_ns == 0 {
            return fallback_fps.max(1);
        }
        let fps = NANOS_PER_SECOND.saturating_add(self.avg_ns / 2) / self.avg_ns;
        fps.clamp(1, 999) as u32
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
mod tests;
