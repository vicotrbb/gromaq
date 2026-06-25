use std::time::Instant;

use gromaq::app::{NativeAppAction, NativeAppConfig, NativeAppLifecycle};
use gromaq::renderer::SurfaceFrameError;

#[test]
fn native_app_lifecycle_does_not_count_failed_redraw_attempts_as_presented_frames() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig {
        exit_after_presented_frames: Some(2),
        exit_after_redraw_attempts: Some(3),
        redraw_until_presented_frame_limit: true,
        ..NativeAppConfig::default()
    });
    lifecycle.on_window_created();

    assert_eq!(
        lifecycle.on_redraw_attempt_finished_at(Instant::now(), false),
        NativeAppAction::RequestRedraw
    );
    assert_eq!(lifecycle.redraw_attempts(), 1);
    assert_eq!(lifecycle.frames_presented(), 0);

    assert_eq!(
        lifecycle.on_redraw_attempt_finished_at(Instant::now(), false),
        NativeAppAction::RequestRedraw
    );
    assert_eq!(lifecycle.redraw_attempts(), 2);
    assert_eq!(lifecycle.frames_presented(), 0);

    assert_eq!(
        lifecycle.on_redraw_attempt_finished_at(Instant::now(), false),
        NativeAppAction::Exit
    );
    assert_eq!(lifecycle.redraw_attempts(), 3);
    assert_eq!(lifecycle.frames_presented(), 0);
    assert!(lifecycle.close_requested());

    let report = lifecycle.run_report();
    assert_eq!(report.redraw_attempts, 3);
    assert_eq!(report.frames_presented, 0);
    assert_eq!(report.frame_interval_samples, 0);
}

#[test]
fn native_app_lifecycle_reports_skipped_surface_frame_acquisition_outcomes() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig::default());

    lifecycle.record_surface_frame_skip(SurfaceFrameError::Timeout);
    lifecycle.record_surface_frame_skip(SurfaceFrameError::Timeout);
    lifecycle.record_surface_frame_skip(SurfaceFrameError::Occluded);
    lifecycle.record_surface_frame_skip(SurfaceFrameError::Lost);

    assert_eq!(lifecycle.surface_frame_timeouts(), 2);
    assert_eq!(lifecycle.surface_frame_occluded(), 1);

    let report = lifecycle.run_report();
    assert_eq!(report.surface_frame_timeouts, 2);
    assert_eq!(report.surface_frame_occluded, 1);
}

#[test]
fn native_app_lifecycle_retries_after_failed_attempts_until_presented_frame_limit() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig {
        exit_after_presented_frames: Some(2),
        exit_after_redraw_attempts: Some(4),
        redraw_until_presented_frame_limit: true,
        ..NativeAppConfig::default()
    });
    let first_presented_at = Instant::now();
    let target_interval = NativeAppConfig::default().target_frame_interval();
    lifecycle.on_window_created();

    assert_eq!(
        lifecycle.on_redraw_attempt_finished_at(first_presented_at, false),
        NativeAppAction::RequestRedraw
    );
    assert_eq!(
        lifecycle.on_redraw_attempt_finished_at(first_presented_at, true),
        NativeAppAction::RequestRedraw
    );
    assert_eq!(
        lifecycle.on_redraw_attempt_finished_at(first_presented_at + target_interval, true),
        NativeAppAction::Exit
    );

    let report = lifecycle.run_report();
    assert_eq!(report.redraw_attempts, 3);
    assert_eq!(report.frames_presented, 2);
    assert_eq!(report.frame_interval_samples, 1);
    assert_eq!(
        report.frame_interval_p95_exact_ns,
        target_interval.as_nanos() as u64
    );
}
