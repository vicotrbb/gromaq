use super::*;

#[test]
fn wide_unicode_occupies_two_cells() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("界").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "界");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn combining_mark_after_wide_unicode_stays_on_wide_leading_cell() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("界\u{0301}").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "界\u{0301}");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.line_text(0), "界\u{0301}");
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn stacked_combining_marks_stay_on_base_cell() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("A\u{0301}\u{0302}B").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "A\u{0301}\u{0302}");
    assert_eq!(grid.cell(0, 1).text, "B");
    assert_eq!(grid.line_text(0), "A\u{0301}\u{0302}B");
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn combining_mark_after_right_edge_print_stays_on_last_cell() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal.write_str("abcd\u{0301}E").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 2).text, "c");
    assert_eq!(grid.cell(0, 3).text, "d\u{0301}");
    assert_eq!(grid.line_text(1), "E");
}
