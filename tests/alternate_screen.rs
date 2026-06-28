use gromaq::{Color, Terminal, TerminalConfig};

#[test]
fn alternate_screen_restores_primary_grid_and_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 3).unwrap());
    terminal.write_str("primary").unwrap();

    terminal.write_str("\x1b[?1049halternate").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "alternate");
    assert_eq!(terminal.dump_cursor().col, 9);

    terminal.write_str("\x1b[?1049l").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "primary");
    assert_eq!(terminal.dump_cursor().col, 7);
}

#[test]
fn alternate_screen_1049_restores_saved_rendition_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("\x1b[31;1mprimary\x1b[?1049h\x1b[0malternate\x1b[?1049lZ")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "primaryZ");
    let restored = grid.cell(0, 7);
    assert_eq!(restored.text, "Z");
    assert_eq!(restored.style.foreground, Color::Ansi(1));
    assert!(restored.style.bold);
}

#[test]
fn alternate_screen_1049_restores_saved_charset_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("\x1b)0\x0eP\x1b[?1049h\x0falt\x1b[?1049lq")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "P─");
    assert_eq!(grid.cell(0, 0).text, "P");
    assert_eq!(grid.cell(0, 1).text, "─");
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn alternate_screen_dec_save_cursor_does_not_replace_primary_saved_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("primary\x1b[?1049halt\x1b7screen\x1b[?1049lZ")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "primaryZ");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 8);
}

#[test]
fn alternate_screen_sco_save_cursor_does_not_replace_primary_saved_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("primary\x1b[s\x1b[?1049halt\x1b[2;4H\x1b[s\x1b[?1049l\x1b[uZ")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "primaryZ");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 8);
}

#[test]
fn alternate_screen_private_mode_save_does_not_replace_primary_saved_mode() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("\x1b[?7l\x1b[?7s\x1b[?1049h\x1b[?7h\x1b[?7s\x1b[?1049l\x1b[?7r\x1b[?7$p")
        .unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1b[?7;2$y");
}

#[test]
fn alternate_screen_mouse_modes_do_not_leak_to_primary_screen() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str(
            "\x1b[?1049h\x1b[?9h\x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1006h\
             \x1b[?1049l\x1b[?9$p\x1b[?1000$p\x1b[?1002$p\x1b[?1003$p\x1b[?1006$p",
        )
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[?9;2$y\x1b[?1000;2$y\x1b[?1002;2$y\x1b[?1003;2$y\x1b[?1006;2$y"
    );
}

#[test]
fn alternate_screen_bracketed_paste_does_not_leak_to_primary_screen() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("\x1b[?1049h\x1b[?2004h\x1b[?1049l\x1b[?2004$p")
        .unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1b[?2004;2$y");
}

#[test]
fn repeated_1049_enter_keeps_original_primary_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("primary\x1b[?1049halternate\x1b[?1049h\x1b[?1049lZ")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "primaryZ");
    assert_eq!(terminal.dump_cursor().col, 8);
}

#[test]
fn alternate_screen_entry_exits_scrollback_view() {
    let mut terminal = Terminal::new(
        TerminalConfig::new(8, 3)
            .unwrap()
            .with_scrollback_limit(8)
            .unwrap(),
    );
    terminal.write_str("one\r\ntwo\r\nthree\r\nfour").unwrap();
    assert!(terminal.scroll_display_up(1));
    assert_eq!(terminal.dump_grid().line_text(0), "one");
    assert!(!terminal.dump_cursor().visible);

    terminal.write_str("\x1b[?1049halt").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "alt");
    assert!(terminal.dump_cursor().visible);
}

#[test]
fn inactive_1049_exit_does_not_restore_unrelated_saved_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal.write_str("abc\x1b[?1048hdef\x1b[?1049lZ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "abcdefZ");
    assert_eq!(terminal.dump_cursor().col, 7);
}

#[test]
fn alternate_screen_does_not_append_to_scrollback() {
    let mut terminal = Terminal::new(
        TerminalConfig::new(8, 2)
            .unwrap()
            .with_scrollback_limit(8)
            .unwrap(),
    );

    terminal.write_str("one\r\ntwo").unwrap();
    terminal
        .write_str("\x1b[?1049halt\r\nlines\r\nignored")
        .unwrap();
    terminal.write_str("\x1b[?1049l\r\nthree").unwrap();

    assert_eq!(terminal.dump_scrollback().lines, vec!["one"]);
    assert_eq!(terminal.dump_grid().line_text(0), "two");
    assert_eq!(terminal.dump_grid().line_text(1), "three");
}

#[test]
fn alternate_screen_resize_reflows_saved_primary_before_restore() {
    let mut terminal = Terminal::new(TerminalConfig::new(6, 2).unwrap());

    terminal.write_str("abcdefghi").unwrap();
    terminal.write_str("\x1b[?1049halt").unwrap();
    terminal.resize(4, 3).unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "alt");

    terminal.write_str("\x1b[?1049l").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cols, 4);
    assert_eq!(grid.rows, 3);
    assert_eq!(grid.line_text(0), "abcd");
    assert_eq!(grid.line_text(1), "efgh");
    assert_eq!(grid.line_text(2), "i");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 3);
}
