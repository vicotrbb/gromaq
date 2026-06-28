use gromaq::{Terminal, TerminalConfig};

#[test]
fn csi_scroll_up_shifts_viewport_without_moving_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 4).unwrap());

    terminal
        .write_str("one\r\ntwo\r\nthree\r\nfour\x1b[2S")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "three");
    assert_eq!(grid.line_text(1), "four");
    assert_eq!(grid.line_text(2), "");
    assert_eq!(grid.line_text(3), "");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 4);
}

#[test]
fn csi_scroll_down_shifts_viewport_without_moving_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 4).unwrap());

    terminal
        .write_str("one\r\ntwo\r\nthree\r\nfour\x1b[2T")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(grid.line_text(2), "one");
    assert_eq!(grid.line_text(3), "two");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 4);
}

#[test]
fn csi_scroll_up_respects_scroll_region() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[2S").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "three");
    assert_eq!(grid.line_text(2), "");
    assert_eq!(grid.line_text(3), "");
    assert_eq!(grid.line_text(4), "bottom");
}

#[test]
fn csi_scroll_down_respects_scroll_region() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[2T").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(grid.line_text(2), "");
    assert_eq!(grid.line_text(3), "one");
    assert_eq!(grid.line_text(4), "bottom");
}

#[test]
fn csi_scroll_down_ecma48_alias_respects_scroll_region() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[2^").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(grid.line_text(2), "");
    assert_eq!(grid.line_text(3), "one");
    assert_eq!(grid.line_text(4), "bottom");
}

#[test]
fn decstbm_constrains_linefeed_scrolling_to_region() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[4;1H\n").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "two");
    assert_eq!(grid.line_text(2), "three");
    assert_eq!(grid.line_text(3), "");
    assert_eq!(grid.line_text(4), "bottom");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 0);
}

#[test]
fn decstbm_zero_parameters_reset_scroll_region_to_full_viewport() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[2;4r\x1b[0;0r\x1bP$qr\x1b\\")
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1bP1$r1;5r\x1b\\"
    );
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 0);
}

#[test]
fn dec_origin_mode_positions_cursor_relative_to_scroll_region() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[2;4r\x1b[?6h\x1b[1;1HZ\x1b[3;1HQ")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "");
    assert_eq!(grid.line_text(1), "Z");
    assert_eq!(grid.line_text(3), "Q");
    assert_eq!(grid.line_text(4), "");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn dec_origin_mode_clamps_cursor_to_scroll_region_bottom() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal.write_str("\x1b[2;4r\x1b[?6h\x1b[9;1HZ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(3), "Z");
    assert_eq!(grid.line_text(4), "");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn dec_origin_mode_clamps_relative_vertical_motion_to_scroll_region() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[2;4r\x1b[?6h\x1b[3;1H\x1b[9A\x1b[1GT\x1b[9B\x1b[1GM")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "");
    assert_eq!(grid.line_text(1), "T");
    assert_eq!(grid.line_text(2), "");
    assert_eq!(grid.line_text(3), "M");
    assert_eq!(grid.line_text(4), "");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn relative_vertical_motion_ignores_scroll_region_when_origin_mode_disabled() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal.write_str("\x1b[2;4r\x1b[1;1H\x1b[9BB").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(3), "");
    assert_eq!(grid.line_text(4), "B");
    assert_eq!(terminal.dump_cursor().row, 4);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn dec_origin_mode_disable_returns_cursor_addressing_to_viewport() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[2;4r\x1b[?6h\x1b[1;1HZ\x1b[?6lQ")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "Q");
    assert_eq!(grid.line_text(1), "Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn dec_save_restore_cursor_restores_origin_mode() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[2;4r\x1b[?6h\x1b7\x1b[?6l\x1b8\x1b[?6$p")
        .unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1b[?6;1$y");
}
