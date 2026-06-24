use std::cell::RefCell;

use gromaq::MemoryClipboard;
use gromaq::cli::{run_with_backend, run_with_backend_and_clipboard};

use super::MockBackend;

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
    assert!(exit.stdout.contains("paste key recognized: true"));
    assert!(exit.stdout.contains("pasted bytes: 30"));
    assert!(exit.stdout.contains("clipboard pastes: 1"));
    assert!(exit.stdout.contains("previous text restored: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(clipboard.read_text().as_deref(), Some("previous clipboard"));
}

#[test]
fn runtime_glyph_frame_smoke_cli_reports_prepared_frame_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-glyph-frame-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime glyph frame smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes: 19"));
    assert!(exit.stdout.contains("planned glyphs:"));
    assert!(exit.stdout.contains("selection backgrounds:"));
    assert!(exit.stdout.contains("renderer atlas hits:"));
    assert!(exit.stdout.contains("renderer atlas misses:"));
    assert!(exit.stdout.contains("renderer atlas entries:"));
    assert!(exit.stdout.contains("rasterized glyphs:"));
    assert!(exit.stdout.contains("prepared quads:"));
    assert!(exit.stdout.contains("background quads:"));
    assert!(exit.stdout.contains("cursor quads:"));
    assert!(exit.stdout.contains("atlas bytes:"));
    assert!(exit.stdout.contains("frame size:"));
    assert!(exit.stdout.contains("line height px: 24"));
    assert!(exit.stdout.contains("surface padding px: 18"));
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
fn runtime_large_output_smoke_cli_reports_rendered_burst_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-large-output-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime large-output smoke: ok"));
    assert!(exit.stdout.contains("lines: 512"));
    assert!(exit.stdout.contains("pumped bytes:"));
    assert!(exit.stdout.contains("scrollback lines: 128"));
    assert!(exit.stdout.contains("rendered frames: 1"));
    assert!(exit.stdout.contains("rendered dirty regions:"));
    assert!(exit.stdout.contains("rendered dirty cells:"));
    assert!(exit.stdout.contains("rendered dirty cells max:"));
    assert!(
        exit.stdout
            .contains("last visible line: gromaq-runtime-line-511")
    );
    assert!(exit.stdout.contains("render p95 ns:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_bounded_state_smoke_cli_reports_capped_long_session_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-bounded-state-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime bounded-state smoke: ok"));
    assert!(exit.stdout.contains("batches: 4"));
    assert!(exit.stdout.contains("lines: 2048"));
    assert!(exit.stdout.contains("pumped bytes:"));
    assert!(exit.stdout.contains("scrollback cap: 128"));
    assert!(exit.stdout.contains("scrollback lines: 128"));
    assert!(exit.stdout.contains("scrollback cell rows: 128"));
    assert!(exit.stdout.contains("scrollback cells:"));
    assert!(exit.stdout.contains("scrollback max cells: 4096"));
    assert!(exit.stdout.contains("rendered frames: 4"));
    assert!(exit.stdout.contains("rendered dirty regions:"));
    assert!(exit.stdout.contains("rendered dirty cells:"));
    assert!(exit.stdout.contains("rendered dirty cells max:"));
    assert!(
        exit.stdout
            .contains("last visible line: gromaq-bounded-line-2047")
    );
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_continuous_output_smoke_cli_reports_streamed_batches_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-continuous-output-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime continuous-output smoke: ok"));
    assert!(exit.stdout.contains("batches: 32"));
    assert!(exit.stdout.contains("lines: 256"));
    assert!(exit.stdout.contains("pumped bytes:"));
    assert!(exit.stdout.contains("scrollback lines: 64"));
    assert!(exit.stdout.contains("rendered frames: 32"));
    assert!(exit.stdout.contains("rendered dirty regions:"));
    assert!(exit.stdout.contains("rendered dirty cells:"));
    assert!(exit.stdout.contains("rendered dirty cells max:"));
    assert!(
        exit.stdout
            .contains("last visible line: gromaq-continuous-line-255")
    );
    assert!(exit.stdout.contains("render p95 ns:"));
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
    assert!(exit.stdout.contains("terminal: 113x31"));
    assert!(exit.stdout.contains("scrollback lines: 96"));
    assert!(exit.stdout.contains("target fps: 120"));
    assert!(exit.stdout.contains("dirty-region rendering: false"));
    assert!(exit.stdout.contains("font size px: 18"));
    assert!(exit.stdout.contains("cell width px: 11"));
    assert!(exit.stdout.contains("line height px: 24"));
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
    assert!(exit.stdout.contains("pumped bytes: 16"));
    assert!(exit.stdout.contains("press reported: true"));
    assert!(exit.stdout.contains("release reported: true"));
    assert!(exit.stdout.contains("wheel reported: true"));
    assert!(exit.stdout.contains("mouse inputs: 3"));
    assert!(exit.stdout.contains("pty input writes: 3"));
    assert!(exit.stdout.contains("pty input bytes: 28"));
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

#[test]
fn runtime_perf_smoke_cli_reports_structured_metrics_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-perf-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime perf smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes: 1"));
    assert!(exit.stdout.contains("rendered frames: 1"));
    assert!(exit.stdout.contains("rendered dirty regions:"));
    assert!(exit.stdout.contains("rendered dirty cells:"));
    assert!(exit.stdout.contains("rendered dirty cells max:"));
    assert!(exit.stdout.contains("render samples: 1"));
    assert!(exit.stdout.contains("render avg ns:"));
    assert!(exit.stdout.contains("render max ns:"));
    assert!(exit.stdout.contains("render p95 ns:"));
    assert!(exit.stdout.contains("input-to-render samples: 1"));
    assert!(exit.stdout.contains("input-to-render avg ns:"));
    assert!(exit.stdout.contains("input-to-render max ns:"));
    assert!(exit.stdout.contains("input-to-render p95 ns:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_perf_p95_smoke_cli_reports_repeated_budget_metrics_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-perf-p95-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime perf p95 smoke: ok"));
    assert!(exit.stdout.contains("samples: 16"));
    assert!(exit.stdout.contains("pumped bytes: 16"));
    assert!(exit.stdout.contains("rendered frames: 16"));
    assert!(exit.stdout.contains("render p95 ns:"));
    assert!(exit.stdout.contains("render p95 budget ns: 6940000"));
    assert!(exit.stdout.contains("input-to-render p95 ns:"));
    assert!(
        exit.stdout
            .contains("input-to-render p95 budget ns: 10000000")
    );
    assert!(exit.stdout.contains("render max ns:"));
    assert!(exit.stdout.contains("input-to-render max ns:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}
