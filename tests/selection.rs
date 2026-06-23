use gromaq::{SelectionRange, Terminal, TerminalConfig};

#[test]
fn copy_selection_returns_text_from_single_visible_row() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("hello world").unwrap();

    terminal.set_selection(SelectionRange::new((0, 1), (0, 4)));

    assert_eq!(terminal.copy_selection().unwrap(), "ello");
}

#[test]
fn copy_selection_returns_single_visible_cell() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("hello").unwrap();

    terminal.set_selection(SelectionRange::new((0, 1), (0, 1)));

    assert_eq!(terminal.copy_selection().unwrap(), "e");
}

#[test]
fn copy_selection_spans_visible_rows_in_grid_order() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("alpha\nbravo\ncharlie").unwrap();

    terminal.set_selection(SelectionRange::new((0, 2), (2, 3)));

    assert_eq!(terminal.copy_selection().unwrap(), "pha\nbravo\nchar");
}

#[test]
fn copy_selection_omits_newline_across_soft_wrapped_rows() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 3).unwrap());
    terminal.write_str("helloworld").unwrap();

    terminal.set_selection(SelectionRange::new((0, 0), (1, 4)));

    assert_eq!(terminal.copy_selection().unwrap(), "helloworld");
}

#[test]
fn copy_selection_preserves_newline_at_hard_breaks() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 3).unwrap());
    terminal.write_str("hello\nworld").unwrap();

    terminal.set_selection(SelectionRange::new((0, 0), (1, 4)));

    assert_eq!(terminal.copy_selection().unwrap(), "hello\nworld");
}

#[test]
fn reversed_selection_is_normalized() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    terminal.write_str("abcdef").unwrap();

    terminal.set_selection(SelectionRange::new((0, 4), (0, 1)));

    assert_eq!(terminal.copy_selection().unwrap(), "bcde");
}

#[test]
fn copy_selection_preserves_wide_cell_text_once() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    terminal.write_str("a界b").unwrap();

    terminal.set_selection(SelectionRange::new((0, 0), (0, 3)));

    assert_eq!(terminal.copy_selection().unwrap(), "a界b");
}

#[test]
fn clearing_selection_removes_copy_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    terminal.write_str("abcdef").unwrap();
    terminal.set_selection(SelectionRange::new((0, 0), (0, 2)));

    terminal.clear_selection();

    assert_eq!(terminal.copy_selection(), None);
}
