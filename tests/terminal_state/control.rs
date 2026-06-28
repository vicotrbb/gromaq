use super::*;

#[test]
fn vertical_tab_and_form_feed_follow_linefeed_without_carriage_return() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 4).unwrap());

    terminal.write_bytes(b"A\x0bB\x0cC").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "A");
    assert_eq!(grid.line_text(1), " B");
    assert_eq!(grid.line_text(2), "  C");
    assert_eq!(terminal.dump_cursor().row, 2);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn shift_out_uses_g1_dec_special_graphics_until_shift_in() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());

    terminal.write_str("\x1b)0A\x0elqk\x0fB").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "A┌─┐B");
    assert_eq!(grid.cell(0, 1).text, "┌");
    assert_eq!(grid.cell(0, 2).text, "─");
    assert_eq!(grid.cell(0, 3).text, "┐");
    assert_eq!(grid.cell(0, 4).text, "B");
}

#[test]
fn dec_cursor_restore_restores_saved_g1_charset_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());

    terminal.write_str("\x1b)0\x0eA\x1b7\x0fB\x1b8q").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "A─");
    assert_eq!(grid.cell(0, 0).text, "A");
    assert_eq!(grid.cell(0, 1).text, "─");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn csi_erase_display_mode_3_clears_scrollback_only() {
    let config = TerminalConfig::new(8, 2)
        .unwrap()
        .with_scrollback_limit(4)
        .unwrap();
    let mut terminal = Terminal::new(config);
    terminal.write_str("one\r\ntwo\r\nthree").unwrap();
    assert_eq!(terminal.dump_scrollback().lines, vec!["one"]);

    terminal.write_str("\x1b[3J").unwrap();

    assert!(terminal.dump_scrollback().lines.is_empty());
    assert_eq!(terminal.dump_grid().line_text(0), "two");
    assert_eq!(terminal.dump_grid().line_text(1), "three");
}

#[test]
fn ris_resets_terminal_state_to_initial_defaults() {
    let config = TerminalConfig::new(8, 3)
        .unwrap()
        .with_scrollback_limit(4)
        .unwrap();
    let mut terminal = Terminal::new(config);

    terminal.write_str("old\r\nscroll\r\nstate\r\n").unwrap();
    assert!(!terminal.dump_scrollback().lines.is_empty());

    terminal
        .write_str("\x1b[31;1;7mA\x1b[?25l\x1b[?1000h\x1b[?1006h\x1b[?2004h\x1b[2;3r")
        .unwrap();
    assert_eq!(
        terminal.encode_paste_text("paste"),
        b"\x1b[200~paste\x1b[201~"
    );
    assert!(
        terminal
            .encode_mouse_event(MouseEvent::new(
                MouseEventKind::Press,
                MouseButton::Left,
                0,
                0
            ))
            .is_some()
    );

    terminal.write_str("\x1bcZ\x1b[3;1H\nQ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(grid.line_text(2), "Q");
    assert_eq!(terminal.dump_scrollback().lines, vec!["Z"]);
    assert_eq!(grid.cell(2, 0).style.foreground, Color::Default);
    assert!(!grid.cell(2, 0).style.bold);
    assert!(!grid.cell(2, 0).style.inverse);
    assert_eq!(terminal.dump_cursor().row, 2);
    assert_eq!(terminal.dump_cursor().col, 1);
    assert!(terminal.dump_cursor().visible);
    assert_eq!(terminal.encode_paste_text("paste"), b"paste");
    assert_eq!(
        terminal.encode_mouse_event(MouseEvent::new(
            MouseEventKind::Press,
            MouseButton::Left,
            0,
            0
        )),
        None
    );
}

#[test]
fn decstr_soft_reset_restores_modes_without_clearing_screen() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str("xy\x1b[1;1H\x1b[31;1m\x1b[?25l\x1b(0\x1b[4h\x1b[!pq")
        .unwrap();

    let grid = terminal.dump_grid();
    let reset_cell = grid.cell(0, 0);
    assert_eq!(grid.line_text(0), "qy");
    assert_eq!(reset_cell.text, "q");
    assert_eq!(reset_cell.style.foreground, Color::Default);
    assert!(!reset_cell.style.bold);
    assert!(terminal.dump_cursor().visible);
}

#[test]
fn decstr_soft_reset_resets_dec_saved_cursor_to_home_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str("\x1b[2;3H\x1b7\x1b[!p\x1b[3;4H\x1b8Z")
        .unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn decstr_soft_reset_disables_autowrap() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal.write_str("\x1b[!pabcdE").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "abcE");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn csi_erase_display_mode_0_clears_from_cursor_to_screen_end() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal
        .write_str("abcd\r\nefgh\r\nijkl\x1b[2;3H\x1b[J")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "abcd");
    assert_eq!(grid.line_text(1), "ef");
    assert_eq!(grid.line_text(2), "");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn csi_erase_display_mode_1_clears_from_screen_start_to_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal
        .write_str("abcd\r\nefgh\r\nijkl\x1b[2;3H\x1b[1J")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "");
    assert_eq!(grid.line_text(1), "   h");
    assert_eq!(grid.line_text(2), "ijkl");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn csi_erase_display_mode_2_clears_screen_without_moving_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal
        .write_str("abcd\r\nefgh\r\nijkl\x1b[2;3H\x1b[2JZ")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "");
    assert_eq!(grid.line_text(1), "  Z");
    assert_eq!(grid.line_text(2), "");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn zsh_style_prompt_repaint_preserves_command_output() {
    let mut terminal = Terminal::new(TerminalConfig::new(80, 8).unwrap());

    terminal
        .write_str(
            "\x1b]2;repo\x07\
             \r\x1b[J\r\n\
             \x1b[A~/Daedalus/gromaq loading ................................ rb 2.7.5 12:56\r\n\
             > \x1b[K\x1b[?1h\x1b=\x1b[?2004h\
             p\x08pwd\
             \x1b[?1l\x1b>\x1b[?25l\x1b[?2004l\
             \r\r\x1b[A\x1b[0m\x1b[27m\x1b[24m\x1b[J\
             \x1b[38;5;76m>\x1b[39m pwd\x1b[K\x1b[?25h\r\r\n\
             /Users/victorbona/Daedalus/gromaq\r\n\
             \x1b]2;repo\x07\
             \r\x1b[0m\x1b[J\r\n\
             \x1b[A~/Daedalus/gromaq ........................................ 12:57\r\n\
             > \x1b[K\x1b[?1h\x1b=\x1b[?2004h",
        )
        .unwrap();

    let grid = terminal.dump_grid();
    let visible = (0..grid.rows)
        .map(|row| grid.line_text(row))
        .collect::<Vec<_>>();

    assert!(
        visible.iter().any(|line| line.contains("> pwd")),
        "visible grid did not retain command line after prompt repaint: {visible:?}"
    );
    assert!(
        visible
            .iter()
            .any(|line| line.contains("/Users/victorbona/Daedalus/gromaq")),
        "visible grid did not retain command output after prompt repaint: {visible:?}"
    );
    assert!(terminal.dump_cursor().visible);
}

#[test]
fn resize_preserves_visible_text_and_clamps_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 2).unwrap());
    terminal.write_str("abc\r\ndef").unwrap();

    terminal.resize(4, 3).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "abc");
    assert_eq!(grid.line_text(1), "def");
    assert!(terminal.dump_cursor().row < 3);
    assert!(terminal.dump_cursor().col < 4);
}

#[test]
fn resize_clamps_saved_sco_cursor_before_restore() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 4).unwrap());

    terminal.write_str("\x1b[4;8H\x1b[s").unwrap();
    terminal.resize(4, 2).unwrap();
    terminal.write_str("\x1b[uZ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(1), "   Z");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn resize_clamps_saved_dec_cursor_before_restore() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 4).unwrap());

    terminal.write_str("\x1b[4;8H\x1b7").unwrap();
    terminal.resize(4, 2).unwrap();
    terminal.write_str("\x1b8Z").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(1), "   Z");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn resize_clears_saved_dec_pending_wrap_before_restore() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal.write_str("\x1b[1;4HA\x1b7").unwrap();
    terminal.resize(5, 2).unwrap();
    terminal.write_str("\x1b8X").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "   X");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 4);
}
