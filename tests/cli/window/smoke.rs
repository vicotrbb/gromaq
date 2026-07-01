use std::cell::RefCell;

use gromaq::cli::{CliExit, NativeAppLaunchConfig};

use super::super::{MockAppLauncher, MockBackend, run_with_backend, run_with_backend_and_app};
use super::{NoGlyphFrameAppLauncher, NoServerTmuxUiAppLauncher, NoTmuxUiFrameAppLauncher};

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
            stdout: "window smoke: ok\npresented frame limit: 3\nredraw attempts: 3\nframes presented: 3\nsurface timeouts: 0\nsurface occluded: 0\nterminal cells: 140x35\ndefault startup content checked: true\ndefault startup marker: tmux Cmd/Ctrl+Shift+T\ntmux status strip rendered: true\ntmux status pane command rendered: true\ntmux manager panel rendered: true\n".to_owned(),
            stderr: String::new(),
        }
    );
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(app.launches.borrow().len(), 1);
    let launch = &app.launches.borrow()[0];
    assert_eq!(launch.app.exit_after_presented_frames, Some(3));
    assert_eq!(launch.app.exit_after_redraw_attempts, Some(16));
    assert!(launch.app.redraw_until_presented_frame_limit);
    assert!(launch.app.tmux_ui_enabled);
    assert!(launch.app.tmux_status_strip_enabled);
    assert!(launch.app.open_tmux_manager_on_start);
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
fn window_smoke_fails_when_default_tmux_ui_is_not_rendered() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = NoTmuxUiFrameAppLauncher;

    let exit = run_with_backend_and_app(["gromaq", "--window-smoke"], &backend, &app);

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(
        exit.stderr.contains(
            "window smoke failed: default tmux UI was not rendered; default startup content checked: false; tmux status strip rendered: false; tmux status pane command rendered: false; tmux manager panel rendered: false"
        )
    );
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn window_smoke_accepts_default_tmux_ui_without_server_pane_command() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = NoServerTmuxUiAppLauncher;

    let exit = run_with_backend_and_app(["gromaq", "--window-smoke"], &backend, &app);

    assert_eq!(exit.code, 0);
    assert!(exit.stderr.is_empty());
    assert!(
        exit.stdout
            .contains("default startup content checked: true")
    );
    assert!(
        exit.stdout
            .contains("tmux status pane command rendered: false")
    );
    assert!(backend.requests.borrow().is_empty());
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
fn window_screenshot_smoke_keeps_native_terminal_window_alive_for_capture() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend_and_app(["gromaq", "--window-screenshot-smoke"], &backend, &app);

    assert_eq!(exit.code, 0);
    assert!(exit.stderr.is_empty());
    assert!(
        exit.stdout
            .starts_with("window screenshot smoke: ok\npresented frame limit: 900\n")
    );
    assert!(
        exit.stdout
            .contains("default startup content checked: true")
    );
    assert!(
        exit.stdout
            .contains("default startup marker: tmux Cmd/Ctrl+Shift+T")
    );
    assert!(exit.stdout.contains("tmux status strip rendered: true"));
    assert!(exit.stdout.contains("tmux manager panel rendered: true"));
    assert_eq!(app.launches.borrow().len(), 1);
    let launch = &app.launches.borrow()[0];
    assert_eq!(launch.app.exit_after_presented_frames, Some(900));
    assert_eq!(launch.app.exit_after_redraw_attempts, Some(3600));
    assert!(launch.app.screen_capture_allowed);
    assert!(launch.app.redraw_until_presented_frame_limit);
    assert_eq!(launch.app.startup_text, None);
    assert_eq!(launch.runtime, NativeAppLaunchConfig::default().runtime);
    assert_eq!(launch.renderer, NativeAppLaunchConfig::default().renderer);
    assert_eq!(launch.config_path, None);
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn window_screenshot_smoke_fails_when_tmux_ui_is_not_rendered() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = NoTmuxUiFrameAppLauncher;

    let exit = run_with_backend_and_app(["gromaq", "--window-screenshot-smoke"], &backend, &app);

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(
        exit.stderr.contains(
            "window screenshot smoke failed: tmux UI was not rendered; default startup content checked: false; tmux status strip rendered: false; tmux manager panel rendered: false"
        )
    );
    assert!(backend.requests.borrow().is_empty());
}
