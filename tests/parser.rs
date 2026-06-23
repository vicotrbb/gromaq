use gromaq::{Color, Terminal, TerminalConfig, UnderlineStyle};

#[test]
fn sgr_sets_and_resets_color_and_bold_attributes() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b[31;1mA\x1b[0mB").unwrap();

    let grid = terminal.dump_grid();
    let red_bold = grid.cell(0, 0);
    assert_eq!(red_bold.text, "A");
    assert_eq!(red_bold.style.foreground, Color::Ansi(1));
    assert!(red_bold.style.bold);

    let plain = grid.cell(0, 1);
    assert_eq!(plain.text, "B");
    assert_eq!(plain.style.foreground, Color::Default);
    assert!(!plain.style.bold);
}

#[test]
fn sgr_sets_and_resets_dim_and_strikethrough_attributes() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b[2;9mA\x1b[22mB\x1b[29mC").unwrap();

    let grid = terminal.dump_grid();
    let dim_struck = grid.cell(0, 0);
    assert_eq!(dim_struck.text, "A");
    assert!(dim_struck.style.dim);
    assert!(dim_struck.style.strikethrough);

    let struck_only = grid.cell(0, 1);
    assert_eq!(struck_only.text, "B");
    assert!(!struck_only.style.dim);
    assert!(struck_only.style.strikethrough);

    let plain = grid.cell(0, 2);
    assert_eq!(plain.text, "C");
    assert!(!plain.style.dim);
    assert!(!plain.style.strikethrough);
}

#[test]
fn sgr_sets_and_resets_blink_hidden_and_overline_attributes() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal
        .write_str("\x1b[5;8;53mA\x1b[25mB\x1b[28mC\x1b[55mD")
        .unwrap();

    let grid = terminal.dump_grid();
    let all_attributes = grid.cell(0, 0).style;
    assert!(all_attributes.blink);
    assert!(all_attributes.hidden);
    assert!(all_attributes.overline);

    let without_blink = grid.cell(0, 1).style;
    assert!(!without_blink.blink);
    assert!(without_blink.hidden);
    assert!(without_blink.overline);

    let without_hidden = grid.cell(0, 2).style;
    assert!(!without_hidden.blink);
    assert!(!without_hidden.hidden);
    assert!(without_hidden.overline);

    let plain = grid.cell(0, 3).style;
    assert!(!plain.blink);
    assert!(!plain.hidden);
    assert!(!plain.overline);
}

#[test]
fn sgr_sets_and_resets_italic_and_inverse_attributes() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b[3;7mA\x1b[23mB\x1b[27mC").unwrap();

    let grid = terminal.dump_grid();
    let italic_inverse = grid.cell(0, 0).style;
    assert!(italic_inverse.italic);
    assert!(italic_inverse.inverse);

    let inverse_only = grid.cell(0, 1).style;
    assert!(!inverse_only.italic);
    assert!(inverse_only.inverse);

    let plain = grid.cell(0, 2).style;
    assert!(!plain.italic);
    assert!(!plain.inverse);
}

#[test]
fn sgr_rapid_blink_sets_blink_attribute_until_reset() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b[6mA\x1b[25mB").unwrap();

    let grid = terminal.dump_grid();
    let rapid_blink = grid.cell(0, 0);
    assert_eq!(rapid_blink.text, "A");
    assert!(rapid_blink.style.blink);

    let plain = grid.cell(0, 1);
    assert_eq!(plain.text, "B");
    assert!(!plain.style.blink);
}

#[test]
fn sgr_accepts_colon_delimited_underline_styles_without_italic_side_effects() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b[4:3mA\x1b[4:4mB\x1b[24mC").unwrap();

    let grid = terminal.dump_grid();
    let curly = grid.cell(0, 0).style;
    assert!(curly.underline);
    assert_eq!(curly.underline_style, UnderlineStyle::Curly);
    assert!(!curly.italic);

    let dotted = grid.cell(0, 1).style;
    assert!(dotted.underline);
    assert_eq!(dotted.underline_style, UnderlineStyle::Dotted);
    assert!(!dotted.italic);

    let plain = grid.cell(0, 2).style;
    assert!(!plain.underline);
    assert_eq!(plain.underline_style, UnderlineStyle::Single);
    assert!(!plain.italic);
}

#[test]
fn unsupported_colon_delimited_underline_style_is_ignored() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b[4:9mA").unwrap();

    let style = terminal.dump_grid().cell(0, 0).style;
    assert!(!style.underline);
    assert_eq!(style.underline_style, UnderlineStyle::Single);
    assert!(!style.strikethrough);
}

#[test]
fn sgr_sets_and_resets_underline_color() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal
        .write_str("\x1b[4;58:2:17:34:51mA\x1b[59mB")
        .unwrap();

    let grid = terminal.dump_grid();
    let colored = grid.cell(0, 0).style;
    assert!(colored.underline);
    assert_eq!(grid.cell_underline_color(0, 0), Color::Rgb(17, 34, 51));

    let default_color = grid.cell(0, 1).style;
    assert!(default_color.underline);
    assert_eq!(grid.cell_underline_color(0, 1), Color::Default);
}

#[test]
fn sgr_accepts_colon_delimited_truecolor_and_indexed_colors() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal
        .write_str("\x1b[38:2:17:34:51;48:5:120mA\x1b[0mB")
        .unwrap();

    let grid = terminal.dump_grid();
    let colored = grid.cell(0, 0);
    assert_eq!(colored.text, "A");
    assert_eq!(colored.style.foreground, Color::Rgb(17, 34, 51));
    assert_eq!(colored.style.background, Color::Indexed(120));

    let plain = grid.cell(0, 1);
    assert_eq!(plain.text, "B");
    assert_eq!(plain.style.foreground, Color::Default);
    assert_eq!(plain.style.background, Color::Default);
}

#[test]
fn sgr_accepts_semicolon_delimited_extended_colors_before_grouped_params() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b[38;5;45;48:2:1:2:3;4:3mA").unwrap();

    let style = terminal.dump_grid().cell(0, 0).style;
    assert_eq!(style.foreground, Color::Indexed(45));
    assert_eq!(style.background, Color::Rgb(1, 2, 3));
    assert!(!style.blink);
    assert!(style.underline);
    assert_eq!(style.underline_style, UnderlineStyle::Curly);
}

#[test]
fn sgr_ignores_out_of_range_extended_color_components() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal
        .write_str("\x1b[31;44mA\x1b[38;5;300mB\x1b[48:2:1:2:999mC")
        .unwrap();

    let grid = terminal.dump_grid();
    let indexed_out_of_range = grid.cell(0, 1);
    assert_eq!(indexed_out_of_range.style.foreground, Color::Ansi(1));
    assert_eq!(indexed_out_of_range.style.background, Color::Ansi(4));

    let truecolor_out_of_range = grid.cell(0, 2);
    assert_eq!(truecolor_out_of_range.style.foreground, Color::Ansi(1));
    assert_eq!(truecolor_out_of_range.style.background, Color::Ansi(4));
}

#[test]
fn sgr_ignores_invalid_grouped_extended_color_params() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal
        .write_str("\x1b[31;44mA\x1b[38:6:1mB\x1b[48:6:5mC")
        .unwrap();

    let grid = terminal.dump_grid();
    let unsupported_foreground_mode = grid.cell(0, 1);
    assert_eq!(unsupported_foreground_mode.style.foreground, Color::Ansi(1));
    assert_eq!(unsupported_foreground_mode.style.background, Color::Ansi(4));
    assert!(!unsupported_foreground_mode.style.bold);

    let unsupported_background_mode = grid.cell(0, 2);
    assert_eq!(unsupported_background_mode.style.foreground, Color::Ansi(1));
    assert_eq!(unsupported_background_mode.style.background, Color::Ansi(4));
    assert!(!unsupported_background_mode.style.blink);
}

#[test]
fn csi_cursor_movement_and_erase_line_are_applied() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("abcd\x1b[2DXY\r\x1b[2KZ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "Z");
    assert_eq!(terminal.dump_cursor().col, 1);
}
