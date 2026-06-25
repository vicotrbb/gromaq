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
fn native_app_lifecycle_schedules_pty_wake_against_monitor_refresh() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig {
        target_fps: 144,
        exit_after_presented_frames: Some(2),
        redraw_until_presented_frame_limit: true,
        ..NativeAppConfig::default()
    });
    let first_presented_at = Instant::now();
    let monitor_frame_interval = Duration::from_nanos(8_333_333);

    lifecycle.on_window_created_with_monitor_refresh(Some(120_000));

    assert_eq!(
        lifecycle.on_redraw_requested_at(first_presented_at),
        NativeAppAction::RequestRedraw
    );
    assert_eq!(
        lifecycle.next_pty_pump_deadline(first_presented_at),
        Some(first_presented_at + monitor_frame_interval)
    );
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
