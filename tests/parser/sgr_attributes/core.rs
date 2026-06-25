use gromaq::{Color, Terminal, TerminalConfig};

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
fn sgr_sets_and_resets_framed_and_encircled_attributes() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b[51mA\x1b[52mB\x1b[54mC").unwrap();

    let grid = terminal.dump_grid();
    let framed = grid.cell(0, 0).style;
    assert!(framed.framed);
    assert!(!framed.encircled);

    let encircled = grid.cell(0, 1).style;
    assert!(!encircled.framed);
    assert!(encircled.encircled);

    let plain = grid.cell(0, 2).style;
    assert!(!plain.framed);
    assert!(!plain.encircled);
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
