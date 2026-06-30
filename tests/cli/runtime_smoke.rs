use std::cell::RefCell;

use gromaq::MemoryClipboard;

use super::{MockBackend, run_with_backend, run_with_backend_and_clipboard};

#[path = "runtime_smoke/committed_text.rs"]
mod committed_text;
#[path = "runtime_smoke/glyph_frame.rs"]
mod glyph_frame;
#[path = "runtime_smoke/osc52_clipboard.rs"]
mod osc52_clipboard;
#[path = "runtime_smoke/output_volume.rs"]
mod output_volume;
#[path = "runtime_smoke/performance.rs"]
mod performance;
#[path = "runtime_smoke/selection_copy.rs"]
mod selection_copy;
#[path = "runtime_smoke/tmux.rs"]
mod tmux;
#[path = "runtime_smoke/tool_workflow.rs"]
mod tool_workflow;
#[test]
fn runtime_clipboard_paste_smoke_cli_routes_clipboard_text_to_runtime_pty() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = MemoryClipboard::new("previous clipboard");

    let exit = run_with_backend_and_clipboard(
        ["gromaq", "--runtime-clipboard-paste-smoke"],
        &backend,
        &mut clipboard,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime clipboard paste smoke: ok"));
    assert!(exit.stdout.contains(
        "recognized native paste shortcuts: dedicated-paste, shift-insert, control-v, super-v"
    ));
    assert!(exit.stdout.contains("pasted bytes: 30"));
    assert!(exit.stdout.contains("clipboard pastes: 1"));
    assert!(exit.stdout.contains("previous text restored: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(clipboard.read_text().as_deref(), Some("previous clipboard"));
}

#[test]
fn runtime_bracketed_paste_smoke_cli_wraps_multiline_utf8_payload_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-bracketed-paste-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime bracketed paste smoke: ok"));
    assert!(exit.stdout.contains("payload bytes: 14"));
    assert!(exit.stdout.contains("encoded bytes: 26"));
    assert!(exit.stdout.contains("paste bytes: 14"));
    assert!(exit.stdout.contains("pty input writes: 1"));
    assert!(exit.stdout.contains("pty input bytes: 26"));
    assert!(exit.stdout.contains("bracketed: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_repaint_smoke_cli_preserves_shell_output_after_prompt_repaint() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-repaint-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime repaint smoke: ok"));
    assert!(exit.stdout.contains("full viewport repainted: true"));
    assert!(exit.stdout.contains("command preserved: true"));
    assert!(exit.stdout.contains("first output row preserved: true"));
    assert!(exit.stdout.contains("second output row preserved: true"));
    assert!(exit.stdout.contains("prompt preserved: true"));
    assert!(exit.stdout.contains("planned glyphs:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_scrollback_smoke_cli_reports_local_history_navigation_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-scrollback-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime scrollback smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes: 32"));
    assert!(exit.stdout.contains("local scroll rows: 4"));
    assert!(exit.stdout.contains("rendered frames: 3"));
    assert!(exit.stdout.contains("rendered dirty regions:"));
    assert!(exit.stdout.contains("rendered dirty cells max:"));
    assert!(
        exit.stdout
            .contains("scrolled visible lines: two|three|four")
    );
    assert!(exit.stdout.contains("live visible lines: four|five|six"));
    assert!(exit.stdout.contains("pty input writes: 0"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_alternate_screen_smoke_cli_reports_restored_primary_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-alternate-screen-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime alternate-screen smoke: ok"));
    assert!(exit.stdout.contains("stages: 3"));
    assert!(exit.stdout.contains("pumped bytes:"));
    assert!(exit.stdout.contains("primary restored: true"));
    assert!(exit.stdout.contains("alternate rendered: true"));
    assert!(
        exit.stdout
            .contains("alternate scrollback suppressed: true")
    );
    assert!(exit.stdout.contains("rendered frames: 3"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_reflow_smoke_cli_reports_resize_reflow_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-reflow-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime reflow smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes:"));
    assert!(exit.stdout.contains("resize events: 1"));
    assert!(exit.stdout.contains("scrollback lines: 2"));
    assert!(
        exit.stdout
            .contains("scrollback hard breaks: [false, true]")
    );
    assert!(exit.stdout.contains("scrollback logical lines: [0, 0]"));
    assert!(exit.stdout.contains("visible lines: klmno|pqrst"));
    assert!(exit.stdout.contains("rendered frames: 1"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_config_reload_smoke_cli_reports_reload_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-config-reload-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime config reload smoke: ok"));
    assert!(exit.stdout.contains("unchanged poll changed: false"));
    assert!(exit.stdout.contains("changed poll changed: true"));
    assert!(exit.stdout.contains("terminal: 104x32"));
    assert!(exit.stdout.contains("scrollback lines: 96"));
    assert!(exit.stdout.contains("target fps: 120"));
    assert!(exit.stdout.contains("dirty-region rendering: false"));
    assert!(exit.stdout.contains("font size px: 18"));
    assert!(exit.stdout.contains("cell width px: 10"));
    assert!(exit.stdout.contains("line height px: 22"));
    assert!(exit.stdout.contains("cell spacing px: 2"));
    assert!(exit.stdout.contains("shell: /bin/sh"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_focus_smoke_cli_reports_focus_events_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-focus-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime focus smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes: 8"));
    assert!(exit.stdout.contains("focus in reported: true"));
    assert!(exit.stdout.contains("focus out reported: true"));
    assert!(exit.stdout.contains("focus inputs: 2"));
    assert!(exit.stdout.contains("pty input writes: 2"));
    assert!(exit.stdout.contains("pty input bytes: 6"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_mouse_smoke_cli_reports_mouse_events_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-mouse-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime mouse smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes: 32"));
    assert!(exit.stdout.contains("press reported: true"));
    assert!(exit.stdout.contains("release reported: true"));
    assert!(exit.stdout.contains("drag reported: true"));
    assert!(exit.stdout.contains("motion reported: true"));
    assert!(exit.stdout.contains("wheel reported: true"));
    assert!(exit.stdout.contains("default press reported: true"));
    assert!(exit.stdout.contains("default release reported: true"));
    assert!(exit.stdout.contains("x10 press reported: true"));
    assert!(exit.stdout.contains("x10 release suppressed: true"));
    assert!(exit.stdout.contains("mouse inputs: 8"));
    assert!(exit.stdout.contains("pty input writes: 8"));
    assert!(exit.stdout.contains("pty input bytes: 66"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_response_smoke_cli_reports_terminal_response_writeback_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-response-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime response smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes: 21"));
    assert!(exit.stdout.contains("response writes: 1"));
    assert!(exit.stdout.contains("response bytes: 26"));
    assert!(exit.stdout.contains("pty input writes: 0"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_idle_smoke_cli_reports_clean_frame_suppression_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-idle-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime idle smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes: 0"));
    assert!(exit.stdout.contains("render attempts: 16"));
    assert!(exit.stdout.contains("clean frame skips: 16"));
    assert!(exit.stdout.contains("rendered frames: 0"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}
