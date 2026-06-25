use gromaq::{Terminal, TerminalConfig};

#[test]
fn horizontal_tab_advances_to_next_default_tab_stop() {
    let mut terminal = Terminal::new(TerminalConfig::new(16, 2).unwrap());

    terminal.write_str("a\tb").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "a       b");
    assert_eq!(terminal.dump_cursor().col, 9);
}

#[test]
fn escape_horizontal_tab_set_adds_custom_tab_stop() {
    let mut terminal = Terminal::new(TerminalConfig::new(16, 2).unwrap());

    terminal.write_str("\x1b[1;6H\x1bH\x1b[1;1H\tZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "     Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 6);
}

#[test]
fn c1_horizontal_tab_set_adds_custom_tab_stop() {
    let mut terminal = Terminal::new(TerminalConfig::new(16, 2).unwrap());

    terminal.write_bytes(b"\x1b[1;6H\x88\x1b[1;1H\tZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "     Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 6);
}

#[test]
fn csi_tab_clear_removes_current_default_tab_stop() {
    let mut terminal = Terminal::new(TerminalConfig::new(16, 2).unwrap());

    terminal.write_str("\x1b[1;9H\x1b[g\x1b[1;1H\tZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "               Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 15);
}

#[test]
fn csi_tab_clear_all_removes_default_tab_stops() {
    let mut terminal = Terminal::new(TerminalConfig::new(16, 2).unwrap());

    terminal.write_str("\x1b[3g\x1b[1;1H\tZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "               Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 15);
}

#[test]
fn csi_cursor_forward_tab_moves_across_default_tab_stops() {
    let mut terminal = Terminal::new(TerminalConfig::new(20, 2).unwrap());

    terminal.write_str("abc\x1b[2IZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "abc             Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 17);
}

#[test]
fn csi_cursor_backward_tab_moves_across_default_tab_stops() {
    let mut terminal = Terminal::new(TerminalConfig::new(20, 2).unwrap());

    terminal.write_str("\x1b[1;18H\x1b[2ZZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "        Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 9);
}

#[test]
fn csi_tab_navigation_clamps_to_viewport_edges() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 2).unwrap());

    terminal.write_str("\x1b[1;9H\x1b[4IZ").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "         Z");
    assert_eq!(terminal.dump_cursor().col, 9);

    terminal.write_str("\x1b[4ZZ").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "Z        Z");
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn dec_special_graphics_charset_maps_box_drawing_and_restores_ascii() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str("\x1b(0lqk\r\nx x\r\nmqj\x1b(Bq")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "┌─┐");
    assert_eq!(grid.line_text(1), "│ │");
    assert_eq!(grid.line_text(2), "└─┘q");
}

#[test]
fn shift_out_invokes_g1_dec_special_graphics_until_shift_in() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b)0\x0elqk\x0fq").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "┌─┐q");
}

#[test]
fn decaln_fills_complete_viewport_with_alignment_pattern() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 3).unwrap());

    terminal
        .write_str("\x1b[2;2r\x1b[31mABCD\r\n界Z\r\nxy\x1b#8")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "EEEE");
    assert_eq!(grid.line_text(1), "EEEE");
    assert_eq!(grid.line_text(2), "EEEE");
}
