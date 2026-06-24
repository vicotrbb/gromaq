use super::*;

#[test]
fn wide_unicode_occupies_two_cells() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("界").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "界");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn combining_mark_after_wide_unicode_stays_on_wide_leading_cell() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("界\u{0301}").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "界\u{0301}");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.line_text(0), "界\u{0301}");
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn stacked_combining_marks_stay_on_base_cell() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("A\u{0301}\u{0302}B").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "A\u{0301}\u{0302}");
    assert_eq!(grid.cell(0, 1).text, "B");
    assert_eq!(grid.line_text(0), "A\u{0301}\u{0302}B");
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn combining_mark_after_right_edge_print_stays_on_last_cell() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal.write_str("abcd\u{0301}E").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 2).text, "c");
    assert_eq!(grid.cell(0, 3).text, "d\u{0301}");
    assert_eq!(grid.line_text(1), "E");
}

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
fn emoji_modifier_sequence_stays_in_one_wide_cell_cluster() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("👍🏽").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "👍🏽");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.line_text(0), "👍🏽");
    assert_eq!(terminal.dump_cursor().col, 2);
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

#[test]
fn variation_selector_emoji_presentation_widens_symbol_cluster() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("☃️Z").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "☃️");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.cell(0, 2).text, "Z");
    assert_eq!(grid.line_text(0), "☃️Z");
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn keycap_emoji_sequence_widens_digit_cluster() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("1️⃣Z").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "1️⃣");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.cell(0, 2).text, "Z");
    assert_eq!(grid.line_text(0), "1️⃣Z");
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn emoji_presentation_at_right_edge_keeps_existing_single_cell_span() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal.write_str("abc☃️Z").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 3).text, "☃️");
    assert!(!grid.cell(0, 3).is_wide_leading);
    assert_eq!(grid.line_text(1), "Z");
}

#[test]
fn regional_indicator_pair_stays_in_one_wide_cell_cluster() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("🇺🇸A").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "🇺🇸");
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.cell(0, 2).text, "A");
    assert_eq!(grid.line_text(0), "🇺🇸A");
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn regional_indicator_pair_after_right_edge_print_stays_clustered() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal.write_str("abc🇺🇸Z").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 2).text, "c");
    assert_eq!(grid.cell(0, 3).text, "🇺🇸");
    assert_eq!(grid.line_text(1), "Z");
}

#[test]
fn tag_sequence_emoji_flag_stays_in_one_wide_cell_cluster() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str("🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}Z")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(
        grid.cell(0, 0).text,
        "🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}"
    );
    assert!(grid.cell(0, 0).is_wide_leading);
    assert!(grid.cell(0, 1).is_wide_trailing);
    assert_eq!(grid.cell(0, 2).text, "Z");
    assert_eq!(
        grid.line_text(0),
        "🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}Z"
    );
    assert_eq!(terminal.dump_cursor().col, 3);
}
