use std::cell::RefCell;

use gromaq::MemoryClipboard;

use crate::{MockBackend, run_with_backend_and_clipboard};

#[test]
fn runtime_osc52_clipboard_smoke_cli_syncs_terminal_clipboard_to_host_clipboard() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = MemoryClipboard::new("previous clipboard");

    let exit = run_with_backend_and_clipboard(
        ["gromaq", "--runtime-osc52-clipboard-smoke"],
        &backend,
        &mut clipboard,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime OSC 52 clipboard smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes: 44"));
    assert!(exit.stdout.contains("decoded bytes: 26"));
    assert!(exit.stdout.contains("clipboard synced: true"));
    assert!(exit.stdout.contains("repeat sync suppressed: true"));
    assert!(exit.stdout.contains("pty input writes: 0"));
    assert!(exit.stdout.contains("previous text restored: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(clipboard.read_text().as_deref(), Some("previous clipboard"));
}
