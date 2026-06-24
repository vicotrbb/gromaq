use std::time::{Duration, Instant};

use super::*;

mod prepared_frame;
mod surface_validation;

#[test]
fn frame_scheduler_dropped_frame_metrics_saturate() {
    let mut scheduler = FrameScheduler::new(1).unwrap();
    scheduler.metrics_mut().dropped_frames = u64::MAX - 1;
    let start = Instant::now();
    scheduler.record_presented(start);

    scheduler.record_presented(start + Duration::from_secs(4));

    assert_eq!(scheduler.metrics().dropped_frames, u64::MAX);
}

#[test]
fn frame_scheduler_presented_frame_metrics_saturate() {
    let mut scheduler = FrameScheduler::new(144).unwrap();
    scheduler.metrics_mut().frames_presented = u64::MAX;

    scheduler.record_presented(Instant::now());

    assert_eq!(scheduler.metrics().frames_presented, u64::MAX);
}
