use std::cell::RefCell;

use super::{MockBackend, run_with_backend};

#[test]
fn runtime_real_shell_smoke_cli_drives_real_shell_through_runtime() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-real-shell-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime real-shell smoke: ok"));
    assert!(exit.stdout.contains("shell: /bin/sh"));
    assert!(exit.stdout.contains("pumped bytes:"));
    assert!(exit.stdout.contains("pty input writes: 1"));
    assert!(exit.stdout.contains("pty input bytes: 47"));
    assert!(exit.stdout.contains("rendered frames:"));
    assert!(exit.stdout.contains("rendered dirty regions:"));
    assert!(exit.stdout.contains("rendered dirty cells max:"));
    assert!(exit.stdout.contains("ready observed: true"));
    assert!(exit.stdout.contains("input echo observed: true"));
    assert!(exit.stdout.contains("exit echo observed: true"));
    assert!(exit.stdout.contains("render p95 ns:"));
    assert!(exit.stdout.contains("input-to-render p95 ns:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_real_shell_large_output_smoke_cli_renders_real_shell_burst() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(
        ["gromaq", "--runtime-real-shell-large-output-smoke"],
        &backend,
    );

    assert_eq!(exit.code, 0);
    assert!(
        exit.stdout
            .contains("runtime real-shell large-output smoke: ok")
    );
    assert!(exit.stdout.contains("shell: /bin/sh"));
    assert!(exit.stdout.contains("lines: 256"));
    assert!(exit.stdout.contains("pumped bytes:"));
    assert!(exit.stdout.contains("scrollback cap: 64"));
    assert!(exit.stdout.contains("scrollback lines: 64"));
    assert!(exit.stdout.contains("rendered frames:"));
    assert!(exit.stdout.contains("rendered dirty regions:"));
    assert!(exit.stdout.contains("rendered dirty cells max:"));
    assert!(exit.stdout.contains("first line evicted: true"));
    assert!(exit.stdout.contains("last line observed: true"));
    assert!(exit.stdout.contains("render p95 ns:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_real_shell_reflow_smoke_cli_resizes_real_shell_output() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-real-shell-reflow-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime real-shell reflow smoke: ok"));
    assert!(exit.stdout.contains("shell: /bin/sh"));
    assert!(exit.stdout.contains("pumped bytes:"));
    assert!(exit.stdout.contains("resize events: 1"));
    assert!(exit.stdout.contains("scrollback lines: 2"));
    assert!(
        exit.stdout
            .contains("scrollback hard breaks: [false, true]")
    );
    assert!(exit.stdout.contains("visible lines: klmno|pqrst"));
    assert!(exit.stdout.contains("rendered frames: 1"));
    assert!(exit.stdout.contains("rendered dirty regions:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}
