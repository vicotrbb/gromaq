use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

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

    let mut output = Vec::new();
    for _ in 0..30 {
        output.extend(session.drain_available_output().unwrap());
        if String::from_utf8_lossy(&output).contains("gromaq-proxy-wakeup") {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(String::from_utf8_lossy(&output).contains("gromaq-proxy-wakeup"));
    assert!(wakeups.load(Ordering::Relaxed) > 0);
}
