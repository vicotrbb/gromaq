use std::time::{Duration, Instant};

use gromaq::renderer::{FrameDecision, FrameScheduler, RenderReason};

#[test]
fn target_interval_for_144hz_is_under_frame_budget() {
    let scheduler = FrameScheduler::new(144).unwrap();

    assert!(scheduler.target_interval() <= Duration::from_nanos(6_944_445));
    assert!(scheduler.target_interval() >= Duration::from_nanos(6_900_000));
}

#[test]
fn first_dirty_frame_renders_immediately() {
    let scheduler = FrameScheduler::new(144).unwrap();
    let now = Instant::now();

    let decision = scheduler.decide(now, true);

    assert_eq!(
        decision,
        FrameDecision::render(RenderReason::FirstDirtyFrame)
    );
}

#[test]
fn clean_idle_frame_is_suppressed() {
    let scheduler = FrameScheduler::new(144).unwrap();
    let now = Instant::now();

    let decision = scheduler.decide(now, false);

    assert_eq!(decision, FrameDecision::idle());
}

#[test]
fn dirty_frame_before_next_interval_waits_remaining_time() {
    let mut scheduler = FrameScheduler::new(144).unwrap();
    let start = Instant::now();
    scheduler.record_presented(start);

    let decision = scheduler.decide(start + Duration::from_millis(2), true);

    assert!(!decision.should_render);
    assert!(decision.wait_for.unwrap() > Duration::from_millis(4));
    assert_eq!(decision.reason, RenderReason::FramePaced);
}

#[test]
fn dirty_frame_after_interval_renders() {
    let mut scheduler = FrameScheduler::new(144).unwrap();
    let start = Instant::now();
    scheduler.record_presented(start);

    let decision = scheduler.decide(start + Duration::from_millis(7), true);

    assert_eq!(decision, FrameDecision::render(RenderReason::Dirty));
}

#[test]
fn late_frame_records_dropped_intervals_when_presented() {
    let mut scheduler = FrameScheduler::new(144).unwrap();
    let start = Instant::now();
    scheduler.record_presented(start);

    scheduler.record_presented(start + Duration::from_millis(30));

    let metrics = scheduler.metrics();
    assert_eq!(metrics.frames_presented, 2);
    assert_eq!(metrics.dropped_frames, 3);
}

#[test]
fn invalid_target_fps_is_rejected() {
    for target_fps in [0, 1_001] {
        let error = FrameScheduler::new(target_fps).unwrap_err();

        assert!(error.to_string().contains("target fps"));
    }
}
