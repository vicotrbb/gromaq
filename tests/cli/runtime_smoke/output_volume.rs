use std::cell::RefCell;

use gromaq::cli::run_with_backend;

use crate::MockBackend;

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
