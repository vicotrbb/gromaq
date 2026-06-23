use gromaq::{SelectionRange, Terminal, TerminalConfig};

#[test]
fn copy_selection_returns_text_from_single_visible_row() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("hello world").unwrap();

    terminal.set_selection(SelectionRange::new((0, 1), (0, 4)).unwrap());

    assert_eq!(terminal.copy_selection().unwrap(), "ello");
}

#[test]
fn copy_selection_spans_visible_rows_in_grid_order() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("alpha\nbravo\ncharlie").unwrap();

    terminal.set_selection(SelectionRange::new((0, 2), (2, 3)).unwrap());

    assert_eq!(terminal.copy_selection().unwrap(), "pha\nbravo\nchar");
}

#[test]
fn reversed_selection_is_normalized() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    terminal.write_str("abcdef").unwrap();

    terminal.set_selection(SelectionRange::new((0, 4), (0, 1)).unwrap());

    assert_eq!(terminal.copy_selection().unwrap(), "bcde");
}

#[test]
fn clearing_selection_removes_copy_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    terminal.write_str("abcdef").unwrap();
    terminal.set_selection(SelectionRange::new((0, 0), (0, 2)).unwrap());

    terminal.clear_selection();

    assert_eq!(terminal.copy_selection(), None);
}
