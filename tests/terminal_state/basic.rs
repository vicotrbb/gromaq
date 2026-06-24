use super::*;

#[test]
fn terminal_config_sets_initial_cursor_style() {
    let config = TerminalConfig::new(8, 3)
        .unwrap()
        .with_cursor_shape(CursorShape::Bar)
        .unwrap()
        .with_cursor_blinking(false)
        .unwrap();
    let terminal = Terminal::new(config);

    let cursor = terminal.dump_cursor();
    assert_eq!(cursor.shape, CursorShape::Bar);
    assert!(!cursor.blinking);
}

#[test]
fn printable_text_is_written_to_grid_and_advances_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("hi").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "hi");
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn default_autowrap_moves_printing_past_right_edge_to_next_row() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal.write_str("abcdE").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "abcd");
    assert_eq!(grid.line_text(1), "E");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn disabled_autowrap_overwrites_rightmost_cell_without_wrapping() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal.write_str("\x1b[?7labcdE").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "abcE");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn disabled_autowrap_wide_character_at_right_edge_uses_single_cell_span() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal.write_str("\x1b[?7labc界").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 3).text, "界");
    assert!(!grid.cell(0, 3).is_wide_leading);
    assert!(!grid.cell(0, 3).is_wide_trailing);
    assert_eq!(grid.line_text(0), "abc界");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn dec_private_mode_restore_restores_saved_autowrap_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal
        .write_str("\x1b[?7s\x1b[?7labcdE\x1b[?7rFG")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "abcF");
    assert_eq!(grid.line_text(1), "G");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn dec_private_mode_restore_restores_saved_focus_report_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal
        .write_str("\x1b[?1004h\x1b[?1004s\x1b[?1004l")
        .unwrap();
    assert_eq!(terminal.encode_focus_event(true), None);

    terminal.write_str("\x1b[?1004r").unwrap();
    assert_eq!(terminal.encode_focus_event(true), Some(b"\x1b[I".to_vec()));
    assert_eq!(terminal.encode_focus_event(false), Some(b"\x1b[O".to_vec()));
}

#[test]
fn byte_input_parses_text_and_escape_sequences_without_string_conversion() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_bytes(b"abcd\x1b[2DXY").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "abXY");
    assert_eq!(terminal.dump_cursor().col, 4);
}
