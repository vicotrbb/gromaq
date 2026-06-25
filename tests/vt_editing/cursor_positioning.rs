use gromaq::{Terminal, TerminalConfig};

#[test]
fn csi_cursor_character_absolute_moves_within_current_row() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("abcdef\r\x1b[4GZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "abcZef");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 4);
}

#[test]
fn csi_horizontal_position_absolute_moves_within_current_row() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("abcdef\r\x1b[5`Z").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "abcdZf");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 5);
}

#[test]
fn csi_horizontal_position_relative_moves_within_current_row() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("ab\x1b[3aZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "ab   Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 6);
}

#[test]
fn csi_vertical_position_absolute_moves_within_current_column() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 4).unwrap());

    terminal.write_str("ab\r\ncd\r\n\x1b[3G\x1b[1dZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "abZ");
    assert_eq!(terminal.dump_grid().line_text(1), "cd");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn csi_vertical_position_relative_moves_within_current_column() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 4).unwrap());

    terminal.write_str("\x1b[2;3H\x1b[2eZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(3), "  Z");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn csi_cursor_next_line_moves_down_and_returns_to_column_zero() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 4).unwrap());

    terminal.write_str("ab\x1b[2EZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "ab");
    assert_eq!(terminal.dump_grid().line_text(2), "Z");
    assert_eq!(terminal.dump_cursor().row, 2);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn csi_cursor_previous_line_moves_up_and_returns_to_column_zero() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 4).unwrap());

    terminal.write_str("\x1b[4;5H!\x1b[2FZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(1), "Z");
    assert_eq!(terminal.dump_grid().line_text(3), "    !");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 1);
}
