use gromaq::{Terminal, TerminalConfig};

#[test]
fn csi_insert_blank_characters_shifts_line_right() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 2).unwrap());

    terminal.write_str("abcde\x1b[1;3H\x1b[2@XY").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "abXYcde");
}

#[test]
fn insert_mode_shifts_printable_characters_right_until_reset() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 2).unwrap());

    terminal
        .write_str("abcde\x1b[1;3H\x1b[4hXY\x1b[4lZ")
        .unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "abXYZde");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 5);
}

#[test]
fn insert_mode_drops_rightmost_cells_instead_of_growing_line() {
    let mut terminal = Terminal::new(TerminalConfig::new(6, 2).unwrap());

    terminal.write_str("abcdef\x1b[1;3H\x1b[4hXY").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "abXYcd");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 4);
}

#[test]
fn csi_delete_characters_shifts_line_left() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 2).unwrap());

    terminal.write_str("abcdef\x1b[1;3H\x1b[2P").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "abef");
}

#[test]
fn csi_delete_characters_clears_split_wide_cell_metadata() {
    let mut terminal = Terminal::new(TerminalConfig::new(6, 2).unwrap());

    terminal.write_str("A界B\x1b[1;2H\x1b[P").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "A B");
    assert!(!grid.cell(0, 1).is_wide_trailing);
}

#[test]
fn csi_insert_blank_characters_clears_split_wide_cell_metadata() {
    let mut terminal = Terminal::new(TerminalConfig::new(6, 2).unwrap());

    terminal.write_str("A界B\x1b[1;3H\x1b[@").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "A   B");
    assert!(!grid.cell(0, 1).is_wide_leading);
    assert!(!grid.cell(0, 3).is_wide_trailing);
}

#[test]
fn csi_erase_characters_blanks_cells_without_shifting_line() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 2).unwrap());

    terminal.write_str("abcdef\x1b[1;3H\x1b[2X").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "ab  ef");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn csi_erase_characters_clears_split_wide_cell_metadata() {
    let mut terminal = Terminal::new(TerminalConfig::new(6, 2).unwrap());

    terminal.write_str("A界B\x1b[1;3H\x1b[X").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "A  B");
    assert!(!grid.cell(0, 1).is_wide_leading);
}

#[test]
fn csi_repeat_preceding_character_replays_last_printable_character() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 2).unwrap());

    terminal.write_str("ab\x1b[3bZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "abbbbZ");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 6);
}

#[test]
fn csi_repeat_after_combining_mark_replays_base_printable_character() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 2).unwrap());

    terminal.write_str("e\u{0301}\x1b[2bZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "e\u{0301}eeZ");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 4);
}

#[test]
fn csi_repeat_after_emoji_modifier_replays_base_printable_character() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 2).unwrap());

    terminal.write_str("👍🏽\x1b[bZ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 0).text, "👍🏽");
    assert_eq!(grid.cell(0, 2).text, "👍");
    assert_eq!(grid.cell(0, 4).text, "Z");
    assert_eq!(grid.line_text(0), "👍🏽👍Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 5);
}

#[test]
fn csi_repeat_preceding_character_defaults_to_one_and_ignores_empty_history() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 2).unwrap());

    terminal.write_str("\x1b[bA\x1b[b").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "AA");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 2);
}
