use gromaq::app::{NativeAppAction, NativeAppConfig, NativeAppLifecycle};

#[test]
fn native_app_lifecycle_requests_window_redraw_and_exit_in_order() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig::default());

    assert_eq!(lifecycle.on_resumed(), NativeAppAction::CreateWindow);
    lifecycle.on_window_created();
    assert_eq!(lifecycle.windows_created(), 1);
    assert!(lifecycle.has_window());

    assert_eq!(lifecycle.on_resumed(), NativeAppAction::None);
    assert_eq!(lifecycle.on_about_to_wait(), NativeAppAction::None);
    assert_eq!(lifecycle.redraw_requests(), 0);
    assert_eq!(
        lifecycle.on_terminal_output_ready(),
        NativeAppAction::RequestRedraw
    );
    assert_eq!(lifecycle.redraw_requests(), 1);

    assert_eq!(lifecycle.on_redraw_requested(), NativeAppAction::None);
    assert_eq!(lifecycle.frames_presented(), 1);

    assert_eq!(lifecycle.on_close_requested(), NativeAppAction::Exit);
    assert!(lifecycle.close_requested());
    assert_eq!(lifecycle.on_destroyed(), NativeAppAction::Exit);
    assert!(!lifecycle.has_window());
}

#[test]
fn native_app_lifecycle_exits_after_configured_presented_frame_limit() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig {
        exit_after_presented_frames: Some(2),
        ..NativeAppConfig::default()
    });

    assert_eq!(lifecycle.on_redraw_requested(), NativeAppAction::None);
    assert!(!lifecycle.close_requested());
    assert_eq!(lifecycle.frames_presented(), 1);

    assert_eq!(lifecycle.on_redraw_requested(), NativeAppAction::Exit);
    assert!(lifecycle.close_requested());
    assert_eq!(lifecycle.frames_presented(), 2);
    assert_eq!(lifecycle.on_about_to_wait(), NativeAppAction::Exit);
}

#[test]
fn native_app_lifecycle_requests_bounded_continuous_redraw_until_frame_limit() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig {
        exit_after_presented_frames: Some(3),
        redraw_until_presented_frame_limit: true,
        ..NativeAppConfig::default()
    });
    let first_presented_at = std::time::Instant::now();
    let target_interval = NativeAppConfig::default().target_frame_interval();

    lifecycle.on_window_created();

    assert_eq!(
        lifecycle.on_redraw_requested_at(first_presented_at),
        NativeAppAction::RequestRedraw
    );
    assert_eq!(lifecycle.frames_presented(), 1);
    assert_eq!(lifecycle.redraw_requests(), 1);
    assert_eq!(
        lifecycle.next_pty_pump_deadline(first_presented_at),
        Some(first_presented_at + target_interval)
    );

    assert_eq!(
        lifecycle.on_redraw_requested_at(first_presented_at + target_interval),
        NativeAppAction::RequestRedraw
    );
    assert_eq!(lifecycle.frames_presented(), 2);
    assert_eq!(lifecycle.redraw_requests(), 2);

    assert_eq!(
        lifecycle.on_redraw_requested_at(first_presented_at + target_interval * 2),
        NativeAppAction::Exit
    );
    assert_eq!(lifecycle.frames_presented(), 3);
    assert_eq!(lifecycle.redraw_requests(), 2);
    assert!(lifecycle.close_requested());

    let report = lifecycle.run_report();
    assert_eq!(report.frames_presented, 3);
    assert_eq!(report.frame_interval_samples, 2);
    assert_eq!(
        report.frame_interval_total_ns,
        target_interval.as_nanos() as u64 * 2
    );
    assert_eq!(
        report.frame_interval_avg_ns,
        target_interval.as_nanos() as u64
    );
    assert_eq!(
        report.frame_interval_max_ns,
        target_interval.as_nanos() as u64
    );
    assert_eq!(report.frame_interval_max_sample_index, 1);
    assert_eq!(report.frame_interval_p95_ns, 8_000_000);
    assert_eq!(
        report.frame_interval_p95_exact_ns,
        target_interval.as_nanos() as u64
    );
    assert_eq!(report.dropped_frames, 0);
}
