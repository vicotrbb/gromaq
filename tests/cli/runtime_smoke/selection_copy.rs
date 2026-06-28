use std::cell::RefCell;

use gromaq::MemoryClipboard;

use crate::{MockBackend, run_with_backend_and_clipboard};

#[test]
fn runtime_selection_copy_smoke_cli_copies_visible_selection_to_clipboard() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = MemoryClipboard::new("previous clipboard");

    let exit = run_with_backend_and_clipboard(
        ["gromaq", "--runtime-selection-copy-smoke"],
        &backend,
        &mut clipboard,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime selection copy smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes: 22"));
    assert!(exit.stdout.contains("copied text bytes: 14"));
    assert!(exit.stdout.contains("clipboard updated: true"));
    assert!(exit.stdout.contains("pty input writes: 0"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(clipboard.read_text().as_deref(), Some("alpha!\nbeta界"));
}
