use gromaq::{Color, Style, Terminal, TerminalConfig, UnderlineStyle};

#[test]
fn terminal_state_matches_ansi_scrollback_and_alternate_screen_golden() {
    let mut terminal = Terminal::new(
        TerminalConfig::new(12, 4)
            .unwrap()
            .with_scrollback_limit(8)
            .unwrap(),
    );

    terminal
        .write_str(
            "\
plain-one\r\n\
\x1b[31;1mred-bold\x1b[0m\r\n\
\x1b]8;;https://gromaq.dev\x1b\\linked\x1b]8;;\x1b\\\r\n\
wide 界\r\n\
fifth\r\n\
\x1b[?1049halt-screen\x1b[?1049lrestored",
        )
        .unwrap();

    assert_eq!(
        format_terminal_golden(&terminal),
        include_str!("fixtures/terminal_golden/ansi_scrollback_alternate.txt")
    );
}

fn format_terminal_golden(terminal: &Terminal) -> String {
    let grid = terminal.dump_grid();
    let cursor = terminal.dump_cursor();
    let scrollback = terminal.dump_scrollback();
    let metrics = terminal.dump_perf_metrics();
    let red_cell = &scrollback.cells[1][0];
    let linked_cell = grid.cell(0, 0);
    let wide_cell = grid.cell(1, 5);
    let wide_trailing = grid.cell(1, 6);

    format!(
        "\
grid:{cols}x{rows}
visible[0]:{line0}
visible[1]:{line1}
visible[2]:{line2}
visible[3]:{line3}
cursor:row={cursor_row},col={cursor_col},visible={cursor_visible},shape={cursor_shape:?},blinking={cursor_blinking}
scrollback:{scrollback_lines:?}
hyperlinks:{hyperlinks:?}
red_cell:text={red_text:?},style={red_style}
linked_cell:text={linked_text:?},hyperlink={linked_hyperlink:?}
wide_cell:text={wide_text:?},leading={wide_leading},trailing_cell_trailing={wide_trailing_flag}
perf:parsed_bytes={parsed_bytes},dirty_cells={dirty_cells},scrolls={scrolls},resizes={resizes},dirty_batches={dirty_batches}
",
        cols = grid.cols,
        rows = grid.rows,
        line0 = grid.line_text(0),
        line1 = grid.line_text(1),
        line2 = grid.line_text(2),
        line3 = grid.line_text(3),
        cursor_row = cursor.row,
        cursor_col = cursor.col,
        cursor_visible = cursor.visible,
        cursor_shape = cursor.shape,
        cursor_blinking = cursor.blinking,
        scrollback_lines = scrollback.lines,
        hyperlinks = grid.hyperlinks,
        red_text = red_cell.text,
        red_style = format_style(red_cell.style),
        linked_text = linked_cell.text,
        linked_hyperlink = grid.cell_hyperlink(0, 0),
        wide_text = wide_cell.text,
        wide_leading = wide_cell.is_wide_leading,
        wide_trailing_flag = wide_trailing.is_wide_trailing,
        parsed_bytes = metrics.parsed_bytes,
        dirty_cells = metrics.dirty_cells,
        scrolls = metrics.scrolls,
        resizes = metrics.resizes,
        dirty_batches = metrics.dirty_region_batches,
    )
}

fn format_style(style: Style) -> String {
    format!(
        "fg={},bg={},bold={},dim={},italic={},underline={},underline_style={},underline_color_id={},blink={},hidden={},inverse={},overline={},strikethrough={}",
        format_color(style.foreground),
        format_color(style.background),
        style.bold,
        style.dim,
        style.italic,
        style.underline,
        format_underline_style(style.underline_style),
        style.underline_color_id,
        style.blink,
        style.hidden,
        style.inverse,
        style.overline,
        style.strikethrough
    )
}

fn format_color(color: Color) -> String {
    match color {
        Color::Default => "default".to_owned(),
        Color::Ansi(index) => format!("ansi-{index}"),
        Color::Indexed(index) => format!("indexed-{index}"),
        Color::Rgb(red, green, blue) => format!("rgb-{red}-{green}-{blue}"),
    }
}

fn format_underline_style(style: UnderlineStyle) -> &'static str {
    match style {
        UnderlineStyle::Single => "single",
        UnderlineStyle::Double => "double",
        UnderlineStyle::Curly => "curly",
        UnderlineStyle::Dotted => "dotted",
        UnderlineStyle::Dashed => "dashed",
    }
}
