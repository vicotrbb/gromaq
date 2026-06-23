use gromaq::{Color, Terminal, TerminalConfig};

#[test]
fn widening_reflows_soft_wrapped_visible_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 4).unwrap());
    terminal.write_str("helloworld").unwrap();

    terminal.resize(10, 3).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "helloworld");
    assert_eq!(grid.line_text(1), "");
}

#[test]
fn narrowing_reflows_visible_text_across_rows() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 3).unwrap());
    terminal.write_str("helloworld").unwrap();

    terminal.resize(5, 4).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "hello");
    assert_eq!(grid.line_text(1), "world");
    assert_eq!(grid.line_text(2), "");
}

#[test]
fn reflow_preserves_hard_newline_boundaries() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 4).unwrap());
    terminal.write_str("abc\r\ndefghij").unwrap();

    terminal.resize(10, 4).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "abc");
    assert_eq!(grid.line_text(1), "defghij");
    assert_eq!(grid.line_text(2), "");
}

#[test]
fn reflow_preserves_hard_newline_after_partial_display_erase() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 4).unwrap());
    terminal.write_str("abc\r\ndef").unwrap();
    terminal.write_str("\x1b[1;2H\x1b[1J").unwrap();

    terminal.resize(10, 4).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "  c");
    assert_eq!(grid.line_text(1), "def");
}

#[test]
fn reflow_drops_hard_newline_after_full_line_erase() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 4).unwrap());
    terminal.write_str("abc\r\ndef").unwrap();
    terminal.write_str("\x1b[1;1H\x1b[K").unwrap();

    terminal.resize(10, 4).unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "def");
}

#[test]
fn reflow_preserves_hard_newline_after_partial_line_erase() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 4).unwrap());
    terminal.write_str("abc\r\ndef").unwrap();
    terminal.write_str("\x1b[1;2H\x1b[1K").unwrap();

    terminal.resize(10, 4).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "  c");
    assert_eq!(grid.line_text(1), "def");
}

#[test]
fn reflow_preserves_cell_styles() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 4).unwrap());
    terminal.write_str("\x1b[31;1mhelloworld\x1b[0m").unwrap();

    terminal.resize(10, 3).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "helloworld");
    for col in 0..10 {
        let cell = grid.cell(0, col);
        assert_eq!(cell.style.foreground, Color::Ansi(1));
        assert!(cell.style.bold);
    }
}

#[test]
fn reflow_preserves_visible_grid_link_and_underline_metadata() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 4).unwrap());
    terminal
        .write_str(
            "\x1b]8;;https://gromaq.dev\x1b\\\x1b[4;58:2:17:34:51mhelloworld\x1b[0m\x1b]8;;\x1b\\",
        )
        .unwrap();

    terminal.resize(10, 3).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "helloworld");
    for col in 0..10 {
        let cell = grid.cell(0, col);
        assert_eq!(grid.cell_hyperlink(0, col), Some("https://gromaq.dev"));
        assert!(cell.style.underline);
        assert_eq!(grid.cell_underline_color(0, col), Color::Rgb(17, 34, 51));
    }
}

#[test]
fn reflow_preserves_wide_cell_metadata() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 4).unwrap());
    terminal.write_str("ab界cd").unwrap();

    terminal.resize(8, 3).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "ab界cd");
    assert_eq!(grid.cell(0, 2).text, "界");
    assert!(grid.cell(0, 2).is_wide_leading);
    assert!(grid.cell(0, 3).is_wide_trailing);
}

#[test]
fn reflow_preserves_regional_indicator_cluster_metadata() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 4).unwrap());
    terminal.write_str("ab🇺🇸cd").unwrap();

    terminal.resize(8, 3).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "ab🇺🇸cd");
    assert_eq!(grid.cell(0, 2).text, "🇺🇸");
    assert!(grid.cell(0, 2).is_wide_leading);
    assert!(grid.cell(0, 3).is_wide_trailing);
}

#[test]
fn reflow_preserves_multi_part_zwj_cluster_metadata() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 4).unwrap());
    terminal.write_str("ab👨\u{200d}👩\u{200d}👧cd").unwrap();

    terminal.resize(8, 3).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "ab👨\u{200d}👩\u{200d}👧cd");
    assert_eq!(grid.cell(0, 2).text, "👨\u{200d}👩\u{200d}👧");
    assert!(grid.cell(0, 2).is_wide_leading);
    assert!(grid.cell(0, 3).is_wide_trailing);
}

#[test]
fn reflow_preserves_emoji_modifier_zwj_cluster_metadata() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 4).unwrap());
    terminal.write_str("ab👩🏽\u{200d}💻cd").unwrap();

    terminal.resize(8, 3).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "ab👩🏽\u{200d}💻cd");
    assert_eq!(grid.cell(0, 2).text, "👩🏽\u{200d}💻");
    assert!(grid.cell(0, 2).is_wide_leading);
    assert!(grid.cell(0, 3).is_wide_trailing);
}

#[test]
fn reflow_preserves_modifier_on_zwj_joined_component_metadata() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 4).unwrap());
    terminal.write_str("ab👨\u{200d}👩🏽cd").unwrap();

    terminal.resize(8, 3).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "ab👨\u{200d}👩🏽cd");
    assert_eq!(grid.cell(0, 2).text, "👨\u{200d}👩🏽");
    assert!(grid.cell(0, 2).is_wide_leading);
    assert!(grid.cell(0, 3).is_wide_trailing);
}

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
