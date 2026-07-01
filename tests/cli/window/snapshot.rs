use std::cell::RefCell;
use std::ffi::OsString;
use std::fs;

use gromaq::cli::NativeAppLaunchConfig;

use super::super::{MockAppLauncher, MockBackend, run_with_backend_and_app, test_cli_config_path};
use super::NoServerTmuxUiSnapshotAppLauncher;

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
    assert!(exit.stdout.contains("tmux status strip rendered: true"));
    assert!(exit.stdout.contains("tmux manager panel rendered: true"));
    let bytes = fs::read(&path).unwrap();
    let _ = fs::remove_file(&path);
    assert_eq!(bytes, b"P6\n1 1\n255\n\x17\x1b$");
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(app.launches.borrow().len(), 1);
    let launch = &app.launches.borrow()[0];
    assert_eq!(launch.app.exit_after_presented_frames, Some(60));
    assert_eq!(launch.app.exit_after_redraw_attempts, Some(60));
    assert!(launch.app.redraw_until_presented_frame_limit);
    assert!(launch.app.open_tmux_manager_on_start);
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
            OsString::from("printf 'gromaq window glyph frame snapshot\\n'; sleep 1")
        ]
    );
}

#[test]
fn window_tmux_manager_snapshot_smoke_writes_artifact() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };
    let path = test_cli_config_path("window-tmux-manager.ppm");

    let exit = run_with_backend_and_app(
        [
            "gromaq",
            "--window-tmux-manager-snapshot",
            &path.to_string_lossy(),
        ],
        &backend,
        &app,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stderr.is_empty());
    assert!(exit.stdout.contains("window tmux manager snapshot: ok"));
    assert!(
        exit.stdout
            .contains("default startup content checked: true")
    );
    assert!(exit.stdout.contains("glyph frame presented: true"));
    assert!(exit.stdout.contains("tmux status strip rendered: true"));
    assert!(
        exit.stdout
            .contains("tmux status pane command rendered: true")
    );
    assert!(exit.stdout.contains("tmux manager panel rendered: true"));
    assert!(exit.stdout.contains("tmux manager sessions: 1"));
    assert!(exit.stdout.contains("tmux manager windows: 1"));
    assert!(exit.stdout.contains("tmux manager panes: 1"));
    assert!(exit.stdout.contains("terminal cells: 140x35"));
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
    assert_eq!(launch.app.startup_text.as_deref(), None);
    assert_eq!(launch.runtime, NativeAppLaunchConfig::default().runtime);
}

#[test]
fn window_tmux_manager_snapshot_accepts_no_server_without_pane_command() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = NoServerTmuxUiSnapshotAppLauncher;
    let path = test_cli_config_path("window-tmux-manager-no-server.ppm");
    let _ = fs::remove_file(&path);

    let exit = run_with_backend_and_app(
        [
            "gromaq",
            "--window-tmux-manager-snapshot",
            &path.to_string_lossy(),
        ],
        &backend,
        &app,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stderr.is_empty());
    assert!(exit.stdout.contains("window tmux manager snapshot: ok"));
    assert!(
        exit.stdout
            .contains("tmux status pane command rendered: false")
    );
    assert!(exit.stdout.contains("tmux manager panes: 0"));
    assert!(path.exists());
    let _ = fs::remove_file(&path);
}

#[test]
fn window_tmux_manager_snapshot_smoke_requires_output_path() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };

    let exit =
        run_with_backend_and_app(["gromaq", "--window-tmux-manager-snapshot"], &backend, &app);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.starts_with("usage: gromaq ["));
    assert!(
        exit.stderr
            .contains("missing snapshot path for --window-tmux-manager-snapshot")
    );
    assert!(backend.requests.borrow().is_empty());
    assert!(app.launches.borrow().is_empty());
}

#[test]
fn window_tmux_manager_snapshot_smoke_rejects_extra_arguments() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };
    let path = test_cli_config_path("window-tmux-manager-extra.ppm");
    let _ = fs::remove_file(&path);

    let exit = run_with_backend_and_app(
        [
            "gromaq",
            "--window-tmux-manager-snapshot",
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
