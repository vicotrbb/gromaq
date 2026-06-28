use gromaq::{Terminal, TerminalConfig};

#[test]
fn csi_insert_and_delete_lines_affect_rows_below_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 4).unwrap());

    terminal.write_str("one\r\ntwo\r\nthree\r\nfour").unwrap();
    terminal.write_str("\x1b[2;1H\x1b[Linserted").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "one");
    assert_eq!(terminal.dump_grid().line_text(1), "inserted");
    assert_eq!(terminal.dump_grid().line_text(2), "two");
    assert_eq!(terminal.dump_grid().line_text(3), "three");

    terminal.write_str("\x1b[3;1H\x1b[M").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "one");
    assert_eq!(terminal.dump_grid().line_text(1), "inserted");
    assert_eq!(terminal.dump_grid().line_text(2), "three");
    assert_eq!(terminal.dump_grid().line_text(3), "");
}

#[test]
fn csi_insert_lines_respects_scroll_region_bottom() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[3;1H\x1b[L").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "one");
    assert_eq!(grid.line_text(2), "");
    assert_eq!(grid.line_text(3), "two");
    assert_eq!(grid.line_text(4), "bottom");
}

#[test]
fn csi_delete_lines_respects_scroll_region_bottom() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[3;1H\x1b[M").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "one");
    assert_eq!(grid.line_text(2), "three");
    assert_eq!(grid.line_text(3), "");
    assert_eq!(grid.line_text(4), "bottom");
}

#[test]
fn csi_insert_lines_ignores_cursor_outside_scroll_region() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal
        .write_str("\x1b[2;4r\x1b[1;1H\x1b[L\x1b[5;1H\x1b[L")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "one");
    assert_eq!(grid.line_text(2), "two");
    assert_eq!(grid.line_text(3), "three");
    assert_eq!(grid.line_text(4), "bottom");
}

#[test]
fn csi_delete_lines_ignores_cursor_outside_scroll_region() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal
        .write_str("\x1b[2;4r\x1b[1;1H\x1b[M\x1b[5;1H\x1b[M")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "one");
    assert_eq!(grid.line_text(2), "two");
    assert_eq!(grid.line_text(3), "three");
    assert_eq!(grid.line_text(4), "bottom");
}

#[test]
fn dec_and_sco_save_restore_cursor_positions() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal.write_str("ab\x1b7cd\x1b8Z").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "abZd");

    terminal.write_str("\x1b[s\x1b[2;5H!\x1b[uQ").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "abZQ");
    assert_eq!(terminal.dump_grid().line_text(1), "    !");
}

#[test]
fn dec_save_restore_cursor_restores_rendition_attributes() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("\x1b[31;1m\x1b7\x1b[0mplain\x1b8Z")
        .unwrap();

    let grid = terminal.dump_grid();
    let restored = grid.cell(0, 0);
    assert_eq!(restored.text, "Z");
    assert_eq!(restored.style.foreground, gromaq::Color::Ansi(1));
    assert!(restored.style.bold);

    let plain = grid.cell(0, 1);
    assert_eq!(plain.text, "l");
    assert_eq!(plain.style.foreground, gromaq::Color::Default);
    assert!(!plain.style.bold);
}

#[test]
fn dec_save_restore_cursor_restores_pending_wrap_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal
        .write_str("\x1b[1;4HA\x1b7\x1b[1;1HB\x1b8X")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "B  A");
    assert_eq!(grid.line_text(1), "X");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn dec_private_1048_saves_and_restores_cursor_without_alternate_screen() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 4).unwrap());

    terminal
        .write_str("\x1b[2;4H\x1b[?1048h\x1b[3;8H!\x1b[?1048lZ")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(1), "   Z");
    assert_eq!(grid.line_text(2), "       !");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 4);
}

#[test]
fn dec_private_1048_restores_rendition_attributes() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("\x1b[34;1m\x1b[?1048h\x1b[0mplain\x1b[?1048lZ")
        .unwrap();

    let grid = terminal.dump_grid();
    let restored = grid.cell(0, 0);
    assert_eq!(restored.text, "Z");
    assert_eq!(restored.style.foreground, gromaq::Color::Ansi(4));
    assert!(restored.style.bold);

    let plain = grid.cell(0, 1);
    assert_eq!(plain.text, "l");
    assert_eq!(plain.style.foreground, gromaq::Color::Default);
    assert!(!plain.style.bold);
}
