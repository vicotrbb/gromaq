use gromaq::{Terminal, TerminalConfig};

#[test]
fn scrollback_view_scrolls_history_into_visible_grid() {
    let mut terminal = Terminal::new(
        TerminalConfig::new(6, 3)
            .unwrap()
            .with_scrollback_limit(8)
            .unwrap(),
    );

    terminal.write_str("one\r\ntwo\r\nthree\r\nfour").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "two");
    assert_eq!(terminal.dump_grid().line_text(1), "three");
    assert_eq!(terminal.dump_grid().line_text(2), "four");
    assert!(terminal.dump_cursor().visible);

    assert!(terminal.scroll_display_up(1));

    let scrolled = terminal.dump_grid();
    assert_eq!(scrolled.line_text(0), "one");
    assert_eq!(scrolled.line_text(1), "two");
    assert_eq!(scrolled.line_text(2), "three");
    assert!(!terminal.dump_cursor().visible);

    assert!(terminal.scroll_display_down(1));

    let live = terminal.dump_grid();
    assert_eq!(live.line_text(0), "two");
    assert_eq!(live.line_text(1), "three");
    assert_eq!(live.line_text(2), "four");
    assert!(terminal.dump_cursor().visible);
}
