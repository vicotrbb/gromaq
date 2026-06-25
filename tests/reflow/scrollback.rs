use gromaq::{Color, Terminal, TerminalConfig};

#[test]
fn narrowing_reflows_existing_scrollback_lines() {
    let config = TerminalConfig::new(10, 2)
        .unwrap()
        .with_scrollback_limit(10)
        .unwrap();
    let mut terminal = Terminal::new(config);
    terminal
        .write_str("abcdefghij\r\nklmnopqrst\r\nuv")
        .unwrap();

    terminal.resize(5, 2).unwrap();

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["abcde", "fghij"]);
}

#[test]
fn scrollback_reflow_preserves_exact_width_hard_line_breaks() {
    let config = TerminalConfig::new(5, 2)
        .unwrap()
        .with_scrollback_limit(10)
        .unwrap();
    let mut terminal = Terminal::new(config);
    terminal.write_str("abcde\r\nfghij\r\nklmno\r\npq").unwrap();

    terminal.resize(10, 2).unwrap();

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["abcde", "fghij"]);
    assert_eq!(scrollback.hard_breaks, vec![true, true]);
    assert_eq!(scrollback.logical_line_ids, vec![0, 1]);
}

#[test]
fn scrollback_reflow_merges_exact_width_soft_wrapped_rows() {
    let config = TerminalConfig::new(5, 2)
        .unwrap()
        .with_scrollback_limit(10)
        .unwrap();
    let mut terminal = Terminal::new(config);
    terminal.write_str("abcdefghijklmnopq").unwrap();

    terminal.resize(10, 2).unwrap();

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["abcdefghij"]);
    assert_eq!(scrollback.hard_breaks, vec![false]);
    assert_eq!(scrollback.logical_line_ids, vec![0]);
}

#[test]
fn scrollback_reflow_keeps_soft_wraps_after_repeated_resize() {
    let config = TerminalConfig::new(5, 2)
        .unwrap()
        .with_scrollback_limit(10)
        .unwrap();
    let mut terminal = Terminal::new(config);
    terminal.write_str("abcdefghijklmnopq").unwrap();

    terminal.resize(4, 2).unwrap();
    terminal.resize(10, 2).unwrap();

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["abcdefghij"]);
    assert_eq!(scrollback.hard_breaks, vec![false]);
    assert_eq!(scrollback.logical_line_ids, vec![0]);
}

#[test]
fn scrollback_reflow_preserves_styled_cell_metadata() {
    let config = TerminalConfig::new(10, 2)
        .unwrap()
        .with_scrollback_limit(10)
        .unwrap();
    let mut terminal = Terminal::new(config);
    terminal
        .write_str("\x1b[31;1mabcdefghij\x1b[0m\r\nklmnopqrst\r\nuv")
        .unwrap();

    terminal.resize(5, 2).unwrap();

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["abcde", "fghij"]);
    assert_eq!(scrollback.logical_line_ids, vec![0, 0]);
    assert_eq!(scrollback.cells.len(), 2);
    for row in 0..2 {
        for col in 0..5 {
            let cell = &scrollback.cells[row][col];
            assert_eq!(cell.style.foreground, Color::Ansi(1));
            assert!(cell.style.bold);
        }
    }
}

#[test]
fn scrollback_reflow_preserves_wide_cluster_metadata() {
    let config = TerminalConfig::new(4, 2)
        .unwrap()
        .with_scrollback_limit(10)
        .unwrap();
    let mut terminal = Terminal::new(config);
    terminal.write_str("ab👨\u{200d}👩\r\ncd\r\nef").unwrap();

    terminal.resize(8, 2).unwrap();

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["ab👨\u{200d}👩"]);
    assert_eq!(scrollback.cells.len(), 1);
    assert_eq!(scrollback.cells[0][2].text, "👨\u{200d}👩");
    assert!(scrollback.cells[0][2].is_wide_leading);
    assert!(scrollback.cells[0][3].is_wide_trailing);
}

#[test]
fn scrollback_reflow_uses_single_cell_wide_span_at_one_column() {
    let config = TerminalConfig::new(4, 2)
        .unwrap()
        .with_scrollback_limit(10)
        .unwrap();
    let mut terminal = Terminal::new(config);
    terminal.write_str("ab界\r\ncd\r\nef").unwrap();

    terminal.resize(1, 2).unwrap();

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["a", "b", "界"]);
    assert_eq!(scrollback.cells[2].len(), 1);
    assert_eq!(scrollback.cells[2][0].text, "界");
    assert!(!scrollback.cells[2][0].is_wide_leading);
    assert!(!scrollback.cells[2][0].is_wide_trailing);
}

#[test]
fn scrollback_reflow_preserves_hyperlink_metadata() {
    let config = TerminalConfig::new(10, 2)
        .unwrap()
        .with_scrollback_limit(10)
        .unwrap();
    let mut terminal = Terminal::new(config);
    terminal
        .write_str("\x1b]8;;https://gromaq.dev\x1b\\abcdefghij\x1b]8;;\x1b\\\r\nklmnopqrst\r\nuv")
        .unwrap();

    terminal.resize(5, 2).unwrap();

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["abcde", "fghij"]);
    assert_eq!(scrollback.hyperlinks, vec!["https://gromaq.dev"]);
    for row in 0..2 {
        for col in 0..5 {
            assert_eq!(scrollback.cells[row][col].hyperlink_id, 1);
        }
    }
}

#[test]
fn scrollback_reflow_preserves_underline_color_metadata() {
    let config = TerminalConfig::new(10, 2)
        .unwrap()
        .with_scrollback_limit(10)
        .unwrap();
    let mut terminal = Terminal::new(config);
    terminal
        .write_str("\x1b[4;58:2:17:34:51mabcdefghij\x1b[0m\r\nklmnopqrst\r\nuv")
        .unwrap();

    terminal.resize(5, 2).unwrap();

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["abcde", "fghij"]);
    assert_eq!(scrollback.underline_colors, vec![Color::Rgb(17, 34, 51)]);
    for row in 0..2 {
        for col in 0..5 {
            let style = scrollback.cells[row][col].style;
            assert!(style.underline);
            assert_eq!(style.underline_color_id, 1);
        }
    }
}
