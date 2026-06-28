use std::cell::RefCell;

use crate::{MockBackend, run_with_backend};

#[test]
fn runtime_committed_text_smoke_cli_routes_utf8_text_to_runtime_pty() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-committed-text-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime committed text smoke: ok"));
    assert!(exit.stdout.contains("committed bytes: 10"));
    assert!(exit.stdout.contains("pty input writes: 1"));
    assert!(exit.stdout.contains("pty input bytes: 10"));
    assert!(exit.stdout.contains("native key inputs: 0"));
    assert!(exit.stdout.contains("paste bytes: 0"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}
