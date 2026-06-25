use gromaq::{Color, Terminal, TerminalConfig, UnderlineStyle};

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
fn sgr_accepts_colon_truecolor_with_colorspace_slot() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal
        .write_str("\x1b[38:2::17:34:51;48:2:0:1:2:3;58:2::4:5:6mA")
        .unwrap();

    let grid = terminal.dump_grid();
    let colored = grid.cell(0, 0);
    assert_eq!(colored.style.foreground, Color::Rgb(17, 34, 51));
    assert_eq!(colored.style.background, Color::Rgb(1, 2, 3));
    assert_eq!(grid.cell_underline_color(0, 0), Color::Rgb(4, 5, 6));
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
fn sgr_invalid_semicolon_truecolor_does_not_leak_components_as_attributes() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal
        .write_str("\x1b[31;44mA\x1b[38;2;999;1;2mB")
        .unwrap();

    let style = terminal.dump_grid().cell(0, 1).style;
    assert_eq!(style.foreground, Color::Ansi(1));
    assert_eq!(style.background, Color::Ansi(4));
    assert!(!style.bold);
    assert!(!style.dim);
}

#[test]
fn sgr_invalid_semicolon_extended_color_mode_does_not_leak_payload_as_attributes() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal
        .write_str("\x1b[31;44mA\x1b[38;6;1mB\x1b[48;6;5mC")
        .unwrap();

    let grid = terminal.dump_grid();
    let foreground_payload = grid.cell(0, 1).style;
    assert_eq!(foreground_payload.foreground, Color::Ansi(1));
    assert_eq!(foreground_payload.background, Color::Ansi(4));
    assert!(!foreground_payload.bold);

    let background_payload = grid.cell(0, 2).style;
    assert_eq!(background_payload.foreground, Color::Ansi(1));
    assert_eq!(background_payload.background, Color::Ansi(4));
    assert!(!background_payload.blink);
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
