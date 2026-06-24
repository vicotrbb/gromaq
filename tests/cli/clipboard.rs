use std::cell::RefCell;

use gromaq::MemoryClipboard;

use super::{MockBackend, ReadOnlyClipboard, run_with_backend_and_clipboard};

#[test]
fn clipboard_smoke_cli_roundtrips_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = MemoryClipboard::new("previous clipboard");

    let exit =
        run_with_backend_and_clipboard(["gromaq", "--clipboard-smoke"], &backend, &mut clipboard);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("clipboard smoke: ok"));
    assert!(exit.stdout.contains("roundtrip bytes: 22"));
    assert!(exit.stdout.contains("previous text restored: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(clipboard.read_text().as_deref(), Some("previous clipboard"));
}

#[test]
fn clipboard_smoke_cli_clears_sentinel_without_previous_text() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = MemoryClipboard::default();

    let exit =
        run_with_backend_and_clipboard(["gromaq", "--clipboard-smoke"], &backend, &mut clipboard);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("clipboard smoke: ok"));
    assert!(exit.stdout.contains("previous text restored: false"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(clipboard.read_text().as_deref(), Some(""));
}

#[test]
fn clipboard_smoke_cli_clears_stale_sentinel_text() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = MemoryClipboard::new("gromaq clipboard smoke");

    let exit =
        run_with_backend_and_clipboard(["gromaq", "--clipboard-smoke"], &backend, &mut clipboard);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("clipboard smoke: ok"));
    assert!(exit.stdout.contains("previous text restored: false"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(clipboard.read_text().as_deref(), Some(""));
}

#[test]
fn clipboard_smoke_cli_reports_readback_mismatch() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = ReadOnlyClipboard {
        text: "unchanged".to_owned(),
    };

    let exit =
        run_with_backend_and_clipboard(["gromaq", "--clipboard-smoke"], &backend, &mut clipboard);

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.contains(
        "clipboard smoke failed: expected \"gromaq clipboard smoke\", read \"unchanged\""
    ));
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn osc52_clipboard_smoke_cli_decodes_and_writes_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = MemoryClipboard::new("previous clipboard");

    let exit = run_with_backend_and_clipboard(
        ["gromaq", "--osc52-clipboard-smoke"],
        &backend,
        &mut clipboard,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("OSC 52 clipboard smoke: ok"));
    assert!(exit.stdout.contains("decoded bytes: 18"));
    assert!(exit.stdout.contains("previous text restored: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(clipboard.read_text().as_deref(), Some("previous clipboard"));
}

#[test]
fn osc52_clipboard_smoke_cli_reports_readback_mismatch() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = ReadOnlyClipboard {
        text: "unchanged".to_owned(),
    };

    let exit = run_with_backend_and_clipboard(
        ["gromaq", "--osc52-clipboard-smoke"],
        &backend,
        &mut clipboard,
    );

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.contains(
        "OSC 52 clipboard smoke failed: expected clipboard text \"gromaq osc52 smoke\", read \"unchanged\""
    ));
    assert!(backend.requests.borrow().is_empty());
}
