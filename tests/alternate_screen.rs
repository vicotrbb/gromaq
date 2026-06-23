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
