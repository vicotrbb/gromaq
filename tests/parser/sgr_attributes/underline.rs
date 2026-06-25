use gromaq::{Color, Terminal, TerminalConfig, UnderlineStyle};

const MAX_METADATA_IDS: u16 = 4096;

#[test]
fn sgr_accepts_colon_delimited_underline_styles_without_italic_side_effects() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal
        .write_str("\x1b[4:1mA\x1b[4:2mB\x1b[4:3mC\x1b[4:4mD\x1b[4:5mE\x1b[4:0mF")
        .unwrap();

    let grid = terminal.dump_grid();
    let single = grid.cell(0, 0).style;
    assert!(single.underline);
    assert_eq!(single.underline_style, UnderlineStyle::Single);
    assert!(!single.italic);

    let double = grid.cell(0, 1).style;
    assert!(double.underline);
    assert_eq!(double.underline_style, UnderlineStyle::Double);
    assert!(!double.italic);

    let curly = grid.cell(0, 2).style;
    assert!(curly.underline);
    assert_eq!(curly.underline_style, UnderlineStyle::Curly);
    assert!(!curly.italic);

    let dotted = grid.cell(0, 3).style;
    assert!(dotted.underline);
    assert_eq!(dotted.underline_style, UnderlineStyle::Dotted);
    assert!(!dotted.italic);

    let dashed = grid.cell(0, 4).style;
    assert!(dashed.underline);
    assert_eq!(dashed.underline_style, UnderlineStyle::Dashed);
    assert!(!dashed.italic);

    let plain = grid.cell(0, 5).style;
    assert!(!plain.underline);
    assert_eq!(plain.underline_style, UnderlineStyle::Single);
    assert!(!plain.italic);
}

#[test]
fn sgr_21_sets_double_underline_until_underline_reset() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b[21mA\x1b[24mB").unwrap();

    let grid = terminal.dump_grid();
    let double = grid.cell(0, 0).style;
    assert!(double.underline);
    assert_eq!(double.underline_style, UnderlineStyle::Double);

    let plain = grid.cell(0, 1).style;
    assert!(!plain.underline);
    assert_eq!(plain.underline_style, UnderlineStyle::Single);
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
fn sgr_underline_color_table_is_bounded_without_panicking() {
    let mut terminal = Terminal::new(TerminalConfig::new(MAX_METADATA_IDS + 2, 1).unwrap());
    let mut input = String::new();

    for index in 0..=MAX_METADATA_IDS {
        let red = ((index >> 8) & 0xff) as u8;
        let green = (index & 0xff) as u8;
        input.push_str(&format!("\x1b[4;58:2:{red}:{green}:0mX"));
    }

    terminal.write_str(&input).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell_underline_color(0, 0), Color::Rgb(0, 0, 0));
    assert_eq!(
        grid.cell_underline_color(0, MAX_METADATA_IDS - 1),
        Color::Rgb(15, 255, 0)
    );
    assert_eq!(
        grid.cell_underline_color(0, MAX_METADATA_IDS),
        Color::Default
    );
}
