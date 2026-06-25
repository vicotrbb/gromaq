use std::cell::RefCell;
use std::ffi::OsString;

use gromaq::cli::NativeAppLaunchConfig;

use super::super::{MockAppLauncher, MockBackend, run_with_backend, run_with_backend_and_app};
use super::{DroppedFrameAppLauncher, NoGlyphFrameAppLauncher};

#[test]
fn window_perf_smoke_launches_bounded_multi_frame_native_terminal_app() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend_and_app(["gromaq", "--window-perf-smoke"], &backend, &app);

    assert_eq!(exit.code, 0);
    assert!(exit.stderr.is_empty());
    assert!(exit.stdout.starts_with(
        "window perf smoke: ok\npresented frame limit: 192\nredraw attempts: 192\nframes presented: 192\nsurface timeouts: 0\nsurface occluded: 0\ntarget fps: 144\nmonitor refresh mhz: 60000\nsurface present mode: Mailbox\nwindow physical size: 2560x1600\nwindow scale milliscale: 2000\nglyph frame presented: true\nglyph frame size: 2560x1600\nglyph frame glyph quads: 12\nglyph frame background quads: 1\nglyph frame decoration quads: 0\nglyph frame cursor quads: 1\nglyph frame atlas bytes: 4096\nglyph frame atlas occupied slots: 8\nframe interval target fps: 60\nframe interval target limited by monitor: true\nframe interval target ns: 16666666\nframe interval p95 budget ns: 20000000\nframe interval warmup frames: 12\nelapsed ns: "
    ));
    assert!(exit.stdout.contains("frame interval samples: 180\n"));
    assert!(exit.stdout.contains("frame interval avg ns: 6940000\n"));
    assert!(exit.stdout.contains("frame interval max ns: 8000000\n"));
    assert!(exit.stdout.contains("frame interval max sample: 17\n"));
    assert!(exit.stdout.contains("frame interval p95 ns: 8000000\n"));
    assert!(
        exit.stdout
            .contains("frame interval p95 exact ns: 8000000\n")
    );
    assert!(exit.stdout.contains("frame intervals over target: 0\n"));
    assert!(
        exit.stdout
            .contains("frame intervals over double target: 0\n")
    );
    assert!(exit.stdout.contains("dropped frames: 0\n"));
    assert!(
        exit.stdout
            .contains("first dropped frame interval sample: 0\n")
    );
    assert!(
        exit.stdout
            .contains("last dropped frame interval sample: 0\n")
    );
    assert!(exit.stdout.contains("frame pacing accepted: true\n"));
    let _elapsed_ns = exit
        .stdout
        .lines()
        .find_map(|line| line.strip_prefix("elapsed ns: "))
        .and_then(|elapsed| elapsed.parse::<u128>().ok())
        .expect("window perf smoke should report elapsed nanoseconds");
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(app.launches.borrow().len(), 1);
    let launch = &app.launches.borrow()[0];
    assert_eq!(launch.app.exit_after_presented_frames, Some(192));
    assert_eq!(launch.app.exit_after_redraw_attempts, Some(768));
    assert!(launch.app.redraw_until_presented_frame_limit);
    assert_eq!(launch.app.frame_interval_warmup_frames, 12);
    assert_eq!(launch.app.target_fps, 144);
    assert_eq!(
        launch.app.startup_text.as_deref(),
        Some("gromaq window perf smoke\nframe pacing probe\n")
    );
    assert_eq!(launch.runtime.shell.program, "/bin/sh");
    assert_eq!(
        launch.runtime.shell.args,
        vec![
            OsString::from("-lc"),
            OsString::from("printf 'gromaq window perf smoke\\nframe pacing probe\\n'")
        ]
    );
    assert_eq!(launch.renderer, NativeAppLaunchConfig::default().renderer);
    assert_eq!(launch.config_path, None);
}

#[test]
fn window_perf_smoke_fails_when_frame_pacing_is_rejected() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = DroppedFrameAppLauncher;

    let exit = run_with_backend_and_app(["gromaq", "--window-perf-smoke"], &backend, &app);

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(
        exit.stderr
            .starts_with("window perf smoke failed: frame pacing was not accepted\n")
    );
    assert!(exit.stderr.contains("glyph frame presented: true\n"));
    assert!(
        exit.stderr
            .contains("frame interval target limited by monitor: true\n")
    );
    assert!(
        exit.stderr
            .contains("frame interval p95 exact ns: 8000000\n")
    );
    assert!(exit.stderr.contains("dropped frames: 1\n"));
    assert!(exit.stderr.contains("frame pacing accepted: false\n"));
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn window_perf_smoke_fails_when_no_glyph_frame_is_presented() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = NoGlyphFrameAppLauncher;

    let exit = run_with_backend_and_app(["gromaq", "--window-perf-smoke"], &backend, &app);

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(
        exit.stderr.contains(
            "window perf smoke failed: no glyph frame was presented; redraw attempts: 768; frames presented: 0; surface timeouts: 0; surface occluded: 768"
        )
    );
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn window_perf_smoke_reports_unavailable_native_app_launcher() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--window-perf-smoke"], &backend);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(
        exit.stderr
            .contains("native app launch unavailable for --window-perf-smoke")
    );
    assert!(backend.requests.borrow().is_empty());
}
