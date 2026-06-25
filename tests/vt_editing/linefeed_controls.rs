use gromaq::{Terminal, TerminalConfig};

#[test]
fn escape_index_scrolls_region_without_carriage_return() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[4;4H\x1bDZ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "two");
    assert_eq!(grid.line_text(2), "three");
    assert_eq!(grid.line_text(3), "   Z");
    assert_eq!(grid.line_text(4), "bottom");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 4);
}

#[test]
fn c1_index_scrolls_region_without_carriage_return() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_bytes(b"\x1b[2;4r\x1b[4;4H\x84Z").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "two");
    assert_eq!(grid.line_text(2), "three");
    assert_eq!(grid.line_text(3), "   Z");
    assert_eq!(grid.line_text(4), "bottom");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 4);
}

#[test]
fn c0_linefeed_controls_move_down_without_carriage_return_by_default() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;4H\nA\x1b[2;6H\x0bB\x1b[3;8H\x0cC")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(1), "   A");
    assert_eq!(grid.line_text(2), "     B");
    assert_eq!(grid.line_text(3), "       C");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 8);
}

#[test]
fn ansi_linefeed_newline_mode_returns_to_column_zero() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal.write_str("\x1b[20h\x1b[1;5H\nZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(1), "Z");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn escape_next_line_scrolls_region_and_returns_to_column_zero() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[4;4H\x1bEZ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "two");
    assert_eq!(grid.line_text(2), "three");
    assert_eq!(grid.line_text(3), "Z");
    assert_eq!(grid.line_text(4), "bottom");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn c1_next_line_scrolls_region_and_returns_to_column_zero() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_bytes(b"\x1b[2;4r\x1b[4;4H\x85Z").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "two");
    assert_eq!(grid.line_text(2), "three");
    assert_eq!(grid.line_text(3), "Z");
    assert_eq!(grid.line_text(4), "bottom");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn reverse_index_at_scroll_top_scrolls_region_down() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[2;1H\x1bM").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(grid.line_text(2), "one");
    assert_eq!(grid.line_text(3), "two");
    assert_eq!(grid.line_text(4), "bottom");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 0);
}

#[test]
fn c1_reverse_index_at_scroll_top_scrolls_region_down() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_bytes(b"\x1b[2;4r\x1b[2;1H\x8d").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(grid.line_text(2), "one");
    assert_eq!(grid.line_text(3), "two");
    assert_eq!(grid.line_text(4), "bottom");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 0);
}

#[test]
fn reverse_index_above_scroll_top_moves_cursor_up_without_scrolling() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[4;1H\x1bMZ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "one");
    assert_eq!(grid.line_text(2), "Zwo");
    assert_eq!(grid.line_text(3), "three");
    assert_eq!(grid.line_text(4), "bottom");
    assert_eq!(terminal.dump_cursor().row, 2);
    assert_eq!(terminal.dump_cursor().col, 1);
}
