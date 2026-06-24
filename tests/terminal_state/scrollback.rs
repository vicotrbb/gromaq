use super::*;

#[test]
fn newline_at_bottom_moves_oldest_line_to_scrollback() {
    let config = TerminalConfig::new(8, 2)
        .unwrap()
        .with_scrollback_limit(4)
        .unwrap();
    let mut terminal = Terminal::new(config);

    terminal.write_str("one\r\ntwo\r\nthree").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "two");
    assert_eq!(grid.line_text(1), "three");
    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["one"]);
}

#[test]
fn long_output_keeps_scrollback_bounded_to_recent_lines() {
    let config = TerminalConfig::new(16, 4)
        .unwrap()
        .with_scrollback_limit(32)
        .unwrap();
    let mut terminal = Terminal::new(config);

    for index in 0..200 {
        terminal
            .write_str(&format!("gromaq-{index:03}\r\n"))
            .unwrap();
    }

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines.len(), 32);
    assert_eq!(scrollback.cells.len(), 32);
    assert_eq!(
        scrollback.lines.first().map(String::as_str),
        Some("gromaq-165")
    );
    assert_eq!(
        scrollback.lines.last().map(String::as_str),
        Some("gromaq-196")
    );
    assert!(!scrollback.lines.iter().any(|line| line == "gromaq-000"));
    assert_eq!(terminal.dump_grid().line_text(1), "gromaq-198");
    assert_eq!(terminal.dump_grid().line_text(2), "gromaq-199");
    assert!(terminal.dump_perf_metrics().scrolls > 32);
}

#[test]
fn scrollback_preserves_wide_cell_metadata_when_row_scrolls_offscreen() {
    let config = TerminalConfig::new(4, 2)
        .unwrap()
        .with_scrollback_limit(4)
        .unwrap();
    let mut terminal = Terminal::new(config);

    terminal.write_str("ab界\r\ncd\r\nef").unwrap();

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["ab界"]);
    assert_eq!(scrollback.cells.len(), 1);
    assert_eq!(scrollback.cells[0][2].text, "界");
    assert!(scrollback.cells[0][2].is_wide_leading);
    assert!(scrollback.cells[0][3].is_wide_trailing);
}

#[test]
fn scrollback_preserves_rich_cell_metadata_when_row_scrolls_offscreen() {
    let config = TerminalConfig::new(8, 2)
        .unwrap()
        .with_scrollback_limit(4)
        .unwrap();
    let mut terminal = Terminal::new(config);

    terminal
        .write_str(
            "\x1b]8;;https://gromaq.dev\x1b\\\x1b[31;4;58:2:17:34:51mabcdefgh\x1b[0m\x1b]8;;\x1b\\\r\nnext\r\nlast",
        )
        .unwrap();

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["abcdefgh"]);
    assert_eq!(scrollback.hyperlinks, vec!["https://gromaq.dev"]);
    assert_eq!(scrollback.underline_colors, vec![Color::Rgb(17, 34, 51)]);
    for cell in &scrollback.cells[0] {
        assert_eq!(cell.hyperlink_id, 1);
        assert_eq!(cell.style.foreground, Color::Ansi(1));
        assert!(cell.style.underline);
        assert_eq!(cell.style.underline_color_id, 1);
    }
}
