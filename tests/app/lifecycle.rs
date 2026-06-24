use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

use gromaq::app::{
    NativeAppAction, NativeAppConfig, NativeAppEvent, NativeAppEventProxy, NativeAppLifecycle,
    NativePtySpawner, RealNativePtySpawner,
};
use gromaq::pty::{PtyConfig, ShellCommand};

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
        NativeAppAction::None
    );
    assert_eq!(lifecycle.frames_presented(), 1);
    assert_eq!(lifecycle.redraw_requests(), 0);
    assert_eq!(
        lifecycle.next_pty_pump_deadline(first_presented_at),
        Some(first_presented_at + target_interval)
    );

    assert_eq!(
        lifecycle.on_about_to_wait_at(first_presented_at + target_interval / 2),
        NativeAppAction::None
    );

    assert_eq!(
        lifecycle.on_about_to_wait_at(first_presented_at + target_interval),
        NativeAppAction::RequestRedraw
    );
    assert_eq!(lifecycle.redraw_requests(), 1);

    assert_eq!(
        lifecycle.on_redraw_requested_at(first_presented_at + target_interval),
        NativeAppAction::None
    );
    assert_eq!(lifecycle.frames_presented(), 2);
    assert_eq!(lifecycle.redraw_requests(), 1);

    assert_eq!(
        lifecycle.on_about_to_wait_at(first_presented_at + target_interval * 2),
        NativeAppAction::RequestRedraw
    );
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
    assert_eq!(report.frame_interval_p95_ns, 8_000_000);
    assert_eq!(report.dropped_frames, 0);
}

#[test]
fn native_app_lifecycle_reports_dropped_presented_frame_intervals() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig {
        exit_after_presented_frames: Some(3),
        redraw_until_presented_frame_limit: true,
        ..NativeAppConfig::default()
    });
    let first_presented_at = Instant::now();

    lifecycle.on_window_created();
    lifecycle.on_redraw_requested_at(first_presented_at);
    lifecycle.on_redraw_requested_at(first_presented_at + Duration::from_nanos(6_944_444));
    lifecycle.on_redraw_requested_at(first_presented_at + Duration::from_nanos(27_777_776));

    let report = lifecycle.run_report();

    assert_eq!(report.frames_presented, 3);
    assert_eq!(report.frame_interval_samples, 2);
    assert_eq!(report.dropped_frames, 2);
}

#[test]
fn native_app_lifecycle_schedules_next_pty_pump_deadline() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig::default());
    let now = std::time::Instant::now();

    assert_eq!(lifecycle.next_pty_pump_deadline(now), None);

    lifecycle.on_window_created();

    assert_eq!(
        lifecycle.next_pty_pump_deadline(now),
        Some(now + NativeAppConfig::default().target_frame_interval())
    );

    lifecycle.on_close_requested();

    assert_eq!(lifecycle.next_pty_pump_deadline(now), None);
}

#[test]
fn native_app_lifecycle_applies_reloaded_frame_cadence() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig::default());
    lifecycle.on_window_created();
    let now = std::time::Instant::now();
    let config = NativeAppConfig {
        target_fps: 120,
        ..NativeAppConfig::default()
    };

    lifecycle.apply_config(config);

    assert_eq!(
        lifecycle.next_pty_pump_deadline(now),
        Some(now + Duration::from_nanos(8_333_333))
    );
}

#[test]
fn native_app_lifecycle_handles_pty_output_ready_user_event() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig::default());

    assert_eq!(
        lifecycle.on_user_event(NativeAppEvent::PtyOutputReady),
        NativeAppAction::None
    );
    assert_eq!(lifecycle.redraw_requests(), 0);

    lifecycle.on_window_created();

    assert_eq!(
        lifecycle.on_user_event(NativeAppEvent::PtyOutputReady),
        NativeAppAction::RequestRedraw
    );
    assert_eq!(lifecycle.redraw_requests(), 1);

    lifecycle.on_close_requested();

    assert_eq!(
        lifecycle.on_user_event(NativeAppEvent::PtyOutputReady),
        NativeAppAction::Exit
    );
}

#[test]
fn real_native_pty_spawner_sends_output_ready_event_when_reader_receives_bytes() {
    let wakeups = Arc::new(AtomicUsize::new(0));
    let wakeups_for_proxy = Arc::clone(&wakeups);
    let proxy = NativeAppEventProxy::from_sender(move |event| {
        if event == NativeAppEvent::PtyOutputReady {
            wakeups_for_proxy.fetch_add(1, Ordering::Relaxed);
        }
    });
    let spawner = RealNativePtySpawner::with_event_proxy(proxy);
    let mut session = spawner
        .spawn(PtyConfig {
            rows: 8,
            cols: 40,
            pixel_width: 0,
            pixel_height: 0,
            shell: ShellCommand {
                program: "/bin/sh".into(),
                args: vec!["-lc".into(), "printf gromaq-proxy-wakeup".into()],
                cwd: None,
            },
        })
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut output = Vec::new();
    while Instant::now() < deadline {
        output.extend(session.drain_available_output().unwrap());
        if String::from_utf8_lossy(&output).contains("gromaq-proxy-wakeup")
            && wakeups.load(Ordering::Relaxed) > 0
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let output = String::from_utf8_lossy(&output);
    let wakeups = wakeups.load(Ordering::Relaxed);
    assert!(
        output.contains("gromaq-proxy-wakeup"),
        "real PTY reader output before deadline: {output:?}"
    );
    assert!(wakeups > 0, "real PTY reader wakeup count: {wakeups}");
}
