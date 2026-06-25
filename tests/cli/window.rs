use std::cell::RefCell;
use std::ffi::OsString;
use std::fs;

use gromaq::app::NativeAppRunReport;
use gromaq::cli::{CliExit, NativeAppLaunchConfig, NativeAppLaunchError, NativeAppLauncher};

use super::{
    MockAppLauncher, MockBackend, run_with_backend, run_with_backend_and_app, test_cli_config_path,
};

#[derive(Debug)]
struct NoGlyphFrameAppLauncher;

#[derive(Debug)]
struct DroppedFrameAppLauncher;

impl NativeAppLauncher for DroppedFrameAppLauncher {
    fn launch(
        &self,
        config: NativeAppLaunchConfig,
    ) -> Result<NativeAppRunReport, NativeAppLaunchError> {
        let frames_presented = config.app.exit_after_presented_frames.unwrap_or_default();
        let warmup_frames = config.app.frame_interval_warmup_frames;
        Ok(NativeAppRunReport {
            redraw_attempts: frames_presented,
            frames_presented,
            monitor_refresh_millihertz: Some(60_000),
            surface_present_mode: Some("Mailbox"),
            window_width_px: Some(2560),
            window_height_px: Some(1600),
            window_scale_milliscale: Some(2000),
            glyph_frame_presented: true,
            glyph_frame_width: 2560,
            glyph_frame_height: 1600,
            glyph_frame_glyph_quads: 12,
            glyph_frame_background_quads: 1,
            glyph_frame_decoration_quads: 0,
            glyph_frame_cursor_quads: 1,
            glyph_frame_atlas_bytes: 4096,
            glyph_frame_atlas_occupied_slots: 8,
            frame_interval_target_fps: 60,
            frame_interval_warmup_frames: warmup_frames,
            frame_interval_samples: frames_presented.saturating_sub(warmup_frames),
            frame_interval_avg_ns: 6_940_000,
            frame_interval_max_ns: 8_000_000,
            frame_interval_max_sample_index: 17,
            frame_interval_p95_ns: 8_000_000,
            frame_interval_p95_exact_ns: 8_000_000,
            frame_intervals_over_target: 2,
            frame_intervals_over_double_target: 0,
            dropped_frames: 1,
            first_dropped_frame_interval_sample: 17,
            last_dropped_frame_interval_sample: 17,
            ..NativeAppRunReport::default()
        })
    }
}

impl NativeAppLauncher for NoGlyphFrameAppLauncher {
    fn launch(
        &self,
        config: NativeAppLaunchConfig,
    ) -> Result<NativeAppRunReport, NativeAppLaunchError> {
        let redraw_attempts = config.app.exit_after_redraw_attempts.unwrap_or_default();
        Ok(NativeAppRunReport {
            redraw_attempts,
            frames_presented: 0,
            surface_frame_occluded: redraw_attempts,
            frame_interval_target_fps: 60,
            frame_interval_warmup_frames: config.app.frame_interval_warmup_frames,
            frame_interval_samples: 0,
            glyph_frame_presented: false,
            ..NativeAppRunReport::default()
        })
    }
}

#[test]
fn window_smoke_launches_bounded_native_terminal_app() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend_and_app(["gromaq", "--window-smoke"], &backend, &app);

    assert_eq!(
        exit,
        CliExit {
            code: 0,
            stdout: "window smoke: ok\npresented frame limit: 1\nredraw attempts: 1\nsurface timeouts: 0\nsurface occluded: 0\n".to_owned(),
            stderr: String::new(),
        }
    );
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(app.launches.borrow().len(), 1);
    let launch = &app.launches.borrow()[0];
    assert_eq!(launch.app.exit_after_presented_frames, Some(1));
    assert_eq!(launch.app.exit_after_redraw_attempts, Some(16));
    assert!(launch.app.redraw_until_presented_frame_limit);
    assert_eq!(launch.runtime, NativeAppLaunchConfig::default().runtime);
    assert_eq!(launch.renderer, NativeAppLaunchConfig::default().renderer);
    assert_eq!(launch.config_path, None);
}

#[test]
fn window_smoke_fails_when_no_surface_frame_is_presented() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = NoGlyphFrameAppLauncher;

    let exit = run_with_backend_and_app(["gromaq", "--window-smoke"], &backend, &app);

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(
        exit.stderr
            .contains("window smoke failed: no surface frame was presented; redraw attempts: 16; surface timeouts: 0; surface occluded: 16")
    );
    assert!(backend.requests.borrow().is_empty());
}

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
fn window_glyph_frame_snapshot_smoke_writes_artifact() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };
    let path = test_cli_config_path("window-glyph-frame.ppm");

    let exit = run_with_backend_and_app(
        [
            "gromaq",
            "--window-glyph-frame-snapshot",
            &path.to_string_lossy(),
        ],
        &backend,
        &app,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stderr.is_empty());
    assert!(exit.stdout.contains("window glyph frame snapshot: ok"));
    assert!(exit.stdout.contains("bytes written: 14"));
    assert!(exit.stdout.contains("frame size: 1x1"));
    assert!(exit.stdout.contains("glyph frame presented: true"));
    let bytes = fs::read(&path).unwrap();
    let _ = fs::remove_file(&path);
    assert_eq!(bytes, b"P6\n1 1\n255\n\x17\x1b$");
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(app.launches.borrow().len(), 1);
    let launch = &app.launches.borrow()[0];
    assert_eq!(launch.app.exit_after_presented_frames, Some(60));
    assert_eq!(launch.app.exit_after_redraw_attempts, Some(60));
    assert!(launch.app.redraw_until_presented_frame_limit);
    assert_eq!(
        launch.app.glyph_frame_snapshot_path.as_deref(),
        Some(path.as_path())
    );
    assert_eq!(
        launch.app.startup_text.as_deref(),
        Some("gromaq window glyph frame snapshot\n")
    );
    assert_eq!(launch.runtime.shell.program, "/bin/sh");
    assert_eq!(
        launch.runtime.shell.args,
        vec![
            OsString::from("-lc"),
            OsString::from("printf 'gromaq window glyph frame snapshot\\n'")
        ]
    );
}

#[test]
fn window_glyph_frame_snapshot_smoke_requires_output_path() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };

    let exit =
        run_with_backend_and_app(["gromaq", "--window-glyph-frame-snapshot"], &backend, &app);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.starts_with("usage: gromaq ["));
    assert!(
        exit.stderr
            .contains("missing snapshot path for --window-glyph-frame-snapshot")
    );
    assert!(backend.requests.borrow().is_empty());
    assert!(app.launches.borrow().is_empty());
}

#[test]
fn window_glyph_frame_snapshot_smoke_rejects_extra_arguments() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };
    let path = test_cli_config_path("window-glyph-frame-extra.ppm");
    let _ = fs::remove_file(&path);

    let exit = run_with_backend_and_app(
        [
            "gromaq",
            "--window-glyph-frame-snapshot",
            &path.to_string_lossy(),
            "extra",
        ],
        &backend,
        &app,
    );

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.starts_with("usage: gromaq ["));
    assert!(exit.stderr.contains("unexpected extra argument: extra"));
    assert!(backend.requests.borrow().is_empty());
    assert!(app.launches.borrow().is_empty());
    assert!(!path.exists());
}

#[test]
fn window_smoke_reports_unavailable_native_app_launcher() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--window-smoke"], &backend);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(
        exit.stderr
            .contains("native app launch unavailable for --window-smoke")
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
