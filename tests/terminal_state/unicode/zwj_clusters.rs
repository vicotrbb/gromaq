use super::*;

#[test]
fn zwj_emoji_sequence_stays_in_one_wide_cell_cluster() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("👨\u{200d}👩").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "👨\u{200d}👩");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.line_text(0), "👨\u{200d}👩");
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn multi_part_zwj_emoji_sequence_stays_in_one_wide_cell_cluster() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("👨\u{200d}👩\u{200d}👧A").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "👨\u{200d}👩\u{200d}👧");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.cell(0, 2).text, "A");
    assert_eq!(grid.line_text(0), "👨\u{200d}👩\u{200d}👧A");
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn zwj_emoji_sequence_with_variation_selector_stays_clustered() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str("👩\u{200d}❤\u{fe0f}\u{200d}💋\u{fe0f}\u{200d}👩Z")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(
        grid.cell(0, 0).text,
        "👩\u{200d}❤\u{fe0f}\u{200d}💋\u{fe0f}\u{200d}👩"
    );
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.cell(0, 2).text, "Z");
    assert_eq!(
        grid.line_text(0),
        "👩\u{200d}❤\u{fe0f}\u{200d}💋\u{fe0f}\u{200d}👩Z"
    );
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn rainbow_flag_zwj_sequence_stays_in_one_wide_cell_cluster() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("🏳️\u{200d}🌈Z").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "🏳️\u{200d}🌈");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.cell(0, 2).text, "Z");
    assert_eq!(grid.line_text(0), "🏳️\u{200d}🌈Z");
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn zwj_emoji_sequence_widens_narrow_symbol_cluster() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("☃\u{200d}❄Z").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "☃\u{200d}❄");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.cell(0, 2).text, "Z");
    assert_eq!(grid.line_text(0), "☃\u{200d}❄Z");
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn emoji_modifier_zwj_sequence_stays_in_one_wide_cell_cluster() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("👩🏽\u{200d}💻Z").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "👩🏽\u{200d}💻");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.cell(0, 2).text, "Z");
    assert_eq!(grid.line_text(0), "👩🏽\u{200d}💻Z");
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn emoji_modifier_after_zwj_joined_component_stays_clustered() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("👨\u{200d}👩🏽Z").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "👨\u{200d}👩🏽");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.cell(0, 2).text, "Z");
    assert_eq!(grid.line_text(0), "👨\u{200d}👩🏽Z");
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn multi_part_zwj_sequence_with_multiple_modifiers_stays_clustered() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("👨🏽\u{200d}👩🏾\u{200d}👧🏼Z").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "👨🏽\u{200d}👩🏾\u{200d}👧🏼");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.cell(0, 2).text, "Z");
    assert_eq!(grid.line_text(0), "👨🏽\u{200d}👩🏾\u{200d}👧🏼Z");
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn zwj_sequence_with_internal_emoji_variation_selector_stays_clustered() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str("👩\u{200d}❤\u{fe0f}\u{200d}💋\u{200d}👨Z")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(
        grid.cell(0, 0).text,
        "👩\u{200d}❤\u{fe0f}\u{200d}💋\u{200d}👨"
    );
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.cell(0, 2).text, "Z");
    assert_eq!(
        grid.line_text(0),
        "👩\u{200d}❤\u{fe0f}\u{200d}💋\u{200d}👨Z"
    );
    assert_eq!(terminal.dump_cursor().col, 3);
}
