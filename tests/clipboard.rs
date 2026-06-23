use gromaq::{MemoryClipboard, NativeClipboard, SelectionRange, Terminal, TerminalConfig};

#[test]
fn copy_selection_writes_plain_text_to_host_clipboard_adapter() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    terminal.write_str("abcdef").unwrap();
    terminal.set_selection(SelectionRange::new((0, 1), (0, 3)));
    let mut clipboard = MemoryClipboard::default();

    let copied = terminal.copy_selection_to_clipboard(&mut clipboard);

    assert_eq!(copied.as_deref(), Some("bcd"));
    assert_eq!(clipboard.read_text().as_deref(), Some("bcd"));
}

#[test]
fn copy_selection_to_clipboard_is_noop_without_selection() {
    let terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    let mut clipboard = MemoryClipboard::new("previous");

    let copied = terminal.copy_selection_to_clipboard(&mut clipboard);

    assert_eq!(copied, None);
    assert_eq!(clipboard.read_text().as_deref(), Some("previous"));
}

#[test]
fn native_clipboard_adapter_can_be_constructed() {
    let _clipboard = NativeClipboard::new();
}
