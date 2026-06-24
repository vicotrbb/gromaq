use std::time::{Duration, Instant};

use super::CliExit;
use crate::renderer::{FrameDecision, FrameScheduler, RenderReason};

pub(super) fn frame_scheduler_smoke_exit() -> CliExit {
    let mut scheduler = match FrameScheduler::new(144) {
        Ok(scheduler) => scheduler,
        Err(error) => return frame_scheduler_smoke_error(error),
    };
    let target_interval = scheduler.target_interval();
    let start = Instant::now();
    let first = scheduler.decide(start, true);
    if first != FrameDecision::render(RenderReason::FirstDirtyFrame) {
        return frame_scheduler_smoke_failure("first dirty frame was not renderable");
    }
    scheduler.record_presented(start);

    let paced = scheduler.decide(start + Duration::from_millis(2), true);
    let Some(wait_for) = paced.wait_for else {
        return frame_scheduler_smoke_failure("dirty frame was not frame-paced before interval");
    };
    if paced.reason != RenderReason::FramePaced {
        return frame_scheduler_smoke_failure("dirty frame was not frame-paced before interval");
    }
    let wait_ns = duration_as_nanos_u64(wait_for);

    let second_presented_at = start + target_interval;
    let second = scheduler.decide(second_presented_at, true);
    if second != FrameDecision::render(RenderReason::Dirty) {
        return frame_scheduler_smoke_failure("dirty frame did not render at target interval");
    }
    scheduler.record_presented(second_presented_at);

    let late_presented_at =
        second_presented_at + target_interval + target_interval + target_interval;
    scheduler.record_presented(late_presented_at);
    let idle = scheduler.decide(late_presented_at + Duration::from_nanos(1), false);
    if idle != FrameDecision::idle() {
        return frame_scheduler_smoke_failure("clean frame was not suppressed");
    }

    let metrics = scheduler.metrics();
    if metrics.frames_presented != 3 || metrics.dropped_frames != 2 {
        return frame_scheduler_smoke_failure("presented-frame metrics did not match timeline");
    }

    CliExit {
        code: 0,
        stdout: format!(
            "frame scheduler smoke: ok\ntarget fps: 144\ntarget interval ns: {}\nframe-paced wait ns: {}\nframes presented: {}\ndropped frames: {}\n",
            duration_as_nanos_u64(target_interval),
            wait_ns,
            metrics.frames_presented,
            metrics.dropped_frames
        ),
        stderr: String::new(),
    }
}

fn duration_as_nanos_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn frame_scheduler_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("frame scheduler smoke failed: {error}\n"),
    }
}

fn frame_scheduler_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("frame scheduler smoke failed: {reason}\n"),
    }
}
