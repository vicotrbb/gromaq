use gromaq::{Terminal, TerminalConfig};

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
fn reflow_preserves_tag_sequence_emoji_flag_metadata() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 4).unwrap());
    terminal
        .write_str("ab🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}cd")
        .unwrap();

    terminal.resize(8, 3).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(
        grid.line_text(0),
        "ab🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}cd"
    );
    assert_eq!(
        grid.cell(0, 2).text,
        "🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}"
    );
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
fn reflow_preserves_rainbow_flag_zwj_cluster_metadata() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 4).unwrap());
    terminal.write_str("ab🏳️\u{200d}🌈cd").unwrap();

    terminal.resize(8, 3).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "ab🏳️\u{200d}🌈cd");
    assert_eq!(grid.cell(0, 2).text, "🏳️\u{200d}🌈");
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
fn reflow_preserves_multi_part_zwj_sequence_with_multiple_modifiers() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 4).unwrap());
    terminal.write_str("ab👨🏽\u{200d}👩🏾\u{200d}👧🏼cd").unwrap();

    terminal.resize(8, 3).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "ab👨🏽\u{200d}👩🏾\u{200d}👧🏼cd");
    assert_eq!(grid.cell(0, 2).text, "👨🏽\u{200d}👩🏾\u{200d}👧🏼");
    assert!(grid.cell(0, 2).is_wide_leading);
    assert!(grid.cell(0, 3).is_wide_trailing);
}

#[test]
fn reflow_preserves_zwj_sequence_with_internal_emoji_variation_selector() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 4).unwrap());
    terminal
        .write_str("ab👩\u{200d}❤\u{fe0f}\u{200d}💋\u{200d}👨cd")
        .unwrap();

    terminal.resize(8, 3).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(
        grid.line_text(0),
        "ab👩\u{200d}❤\u{fe0f}\u{200d}💋\u{200d}👨cd"
    );
    assert_eq!(
        grid.cell(0, 2).text,
        "👩\u{200d}❤\u{fe0f}\u{200d}💋\u{200d}👨"
    );
    assert!(grid.cell(0, 2).is_wide_leading);
    assert!(grid.cell(0, 3).is_wide_trailing);
}
