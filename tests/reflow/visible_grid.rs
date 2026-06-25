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
