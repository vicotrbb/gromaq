use std::time::{Duration, Instant};

use crate::config::MAX_TARGET_FPS;
use crate::error::{GromaqError, Result};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Reason a frame decision was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderReason {
    /// No work is pending.
    Idle,
    /// First frame after dirty state appears.
    FirstDirtyFrame,
    /// Dirty state is pending and the frame interval has elapsed.
    Dirty,
    /// Dirty state exists but the scheduler is waiting for the next frame boundary.
    FramePaced,
}

/// Deterministic frame-scheduling decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDecision {
    /// Whether the renderer should draw now.
    pub should_render: bool,
    /// Optional wait duration before rendering should be reconsidered.
    pub wait_for: Option<Duration>,
    /// Decision reason.
    pub reason: RenderReason,
}

impl FrameDecision {
    /// Build a render-now decision.
    pub fn render(reason: RenderReason) -> Self {
        Self {
            should_render: true,
            wait_for: None,
            reason,
        }
    }

    /// Build an idle decision.
    pub fn idle() -> Self {
        Self {
            should_render: false,
            wait_for: None,
            reason: RenderReason::Idle,
        }
    }

    fn wait(wait_for: Duration) -> Self {
        Self {
            should_render: false,
            wait_for: Some(wait_for),
            reason: RenderReason::FramePaced,
        }
    }
}

/// Frame pacing metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameSchedulerMetrics {
    /// Number of frames marked as presented.
    pub frames_presented: u64,
    /// Number of frame intervals missed between presented frames.
    pub dropped_frames: u64,
}

/// Deterministic frame scheduler for render-loop tests and native UI integration.
#[derive(Debug, Clone)]
pub struct FrameScheduler {
    target_interval: Duration,
    last_presented: Option<Instant>,
    metrics: FrameSchedulerMetrics,
}

impl FrameScheduler {
    /// Create a frame scheduler for `target_fps`.
    pub fn new(target_fps: u32) -> Result<Self> {
        if !(1..=MAX_TARGET_FPS).contains(&target_fps) {
            return Err(GromaqError::InvalidTargetFps {
                minimum: 1,
                maximum: MAX_TARGET_FPS,
                actual: target_fps,
            });
        }
        Ok(Self {
            target_interval: Duration::from_nanos(NANOS_PER_SECOND / u64::from(target_fps)),
            last_presented: None,
            metrics: FrameSchedulerMetrics::default(),
        })
    }

    /// Target interval between presented frames.
    pub fn target_interval(&self) -> Duration {
        self.target_interval
    }

    /// Decide whether a frame should be rendered at `now`.
    pub fn decide(&self, now: Instant, has_dirty: bool) -> FrameDecision {
        if !has_dirty {
            return FrameDecision::idle();
        }
        let Some(last_presented) = self.last_presented else {
            return FrameDecision::render(RenderReason::FirstDirtyFrame);
        };
        let elapsed = now.saturating_duration_since(last_presented);
        if elapsed >= self.target_interval {
            FrameDecision::render(RenderReason::Dirty)
        } else {
            FrameDecision::wait(self.target_interval - elapsed)
        }
    }

    /// Record that a frame was presented at `presented_at`.
    pub fn record_presented(&mut self, presented_at: Instant) {
        if let Some(last_presented) = self.last_presented {
            let elapsed = presented_at.saturating_duration_since(last_presented);
            let intervals = elapsed.as_nanos() / self.target_interval.as_nanos();
            if intervals > 1 {
                self.metrics.dropped_frames = self
                    .metrics
                    .dropped_frames
                    .saturating_add(saturating_u128_to_u64(intervals - 1));
            }
        }
        self.last_presented = Some(presented_at);
        self.metrics.frames_presented = self.metrics.frames_presented.saturating_add(1);
    }

    /// Return scheduler metrics.
    pub fn metrics(&self) -> FrameSchedulerMetrics {
        self.metrics
    }

    #[cfg(test)]
    /// Return mutable scheduler metrics for saturation tests.
    pub(crate) fn metrics_mut(&mut self) -> &mut FrameSchedulerMetrics {
        &mut self.metrics
    }
}

fn saturating_u128_to_u64(value: u128) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
