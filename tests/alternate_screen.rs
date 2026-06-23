use gromaq::{Terminal, TerminalConfig};

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
fn alternate_screen_does_not_append_to_scrollback() {
    let mut terminal = Terminal::new(
        TerminalConfig::new(8, 2)
            .unwrap()
            .with_scrollback_limit(8)
            .unwrap(),
    );

    terminal.write_str("one\ntwo").unwrap();
    terminal
        .write_str("\x1b[?1049halt\nlines\nignored")
        .unwrap();
    terminal.write_str("\x1b[?1049l\nthree").unwrap();

    assert_eq!(terminal.dump_scrollback().lines, vec!["one"]);
    assert_eq!(terminal.dump_grid().line_text(0), "two");
    assert_eq!(terminal.dump_grid().line_text(1), "three");
}
