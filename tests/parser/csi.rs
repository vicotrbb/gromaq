use gromaq::{Terminal, TerminalConfig};

#[test]
fn csi_cursor_movement_and_erase_line_are_applied() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("abcd\x1b[2DXY\r\x1b[2KZ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "Z");
    assert_eq!(terminal.dump_cursor().col, 1);
}
