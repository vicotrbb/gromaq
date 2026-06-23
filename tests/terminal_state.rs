use gromaq::{Color, MouseButton, MouseEvent, MouseEventKind, Terminal, TerminalConfig};

#[test]
fn printable_text_is_written_to_grid_and_advances_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("hi").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "hi");
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn default_autowrap_moves_printing_past_right_edge_to_next_row() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal.write_str("abcdE").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "abcd");
    assert_eq!(grid.line_text(1), "E");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn disabled_autowrap_overwrites_rightmost_cell_without_wrapping() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal.write_str("\x1b[?7labcdE").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "abcE");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn disabled_autowrap_wide_character_at_right_edge_uses_single_cell_span() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal.write_str("\x1b[?7labc界").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell(0, 3).text, "界");
    assert!(!grid.cell(0, 3).is_wide_leading);
    assert!(!grid.cell(0, 3).is_wide_trailing);
    assert_eq!(grid.line_text(0), "abc界");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn dec_private_mode_restore_restores_saved_autowrap_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal
        .write_str("\x1b[?7s\x1b[?7labcdE\x1b[?7rFG")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "abcF");
    assert_eq!(grid.line_text(1), "G");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn dec_private_mode_restore_restores_saved_focus_report_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal
        .write_str("\x1b[?1004h\x1b[?1004s\x1b[?1004l")
        .unwrap();
    assert_eq!(terminal.encode_focus_event(true), None);

    terminal.write_str("\x1b[?1004r").unwrap();
    assert_eq!(terminal.encode_focus_event(true), Some(b"\x1b[I".to_vec()));
    assert_eq!(terminal.encode_focus_event(false), Some(b"\x1b[O".to_vec()));
}

#[test]
fn byte_input_parses_text_and_escape_sequences_without_string_conversion() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_bytes(b"abcd\x1b[2DXY").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "abXY");
    assert_eq!(terminal.dump_cursor().col, 4);
}

#[test]
fn device_status_reports_are_queued_as_terminal_responses() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("\x1b[2;4H\x1b[6n\x1b[5n").unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1b[2;4R\x1b[0n");
    assert!(terminal.take_pending_response_bytes().is_empty());
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn dec_private_cursor_position_report_includes_private_marker() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("\x1b[2;4H\x1b[?6n").unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1b[?2;4R");
}

#[test]
fn dec_private_capability_status_reports_are_queued_as_terminal_responses() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str("\x1b[?15n\x1b[?25n\x1b[?26n\x1b[?53n")
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[?11n\x1b[?20n\x1b[?27;1;0;0n\x1b[?50n"
    );
}

#[test]
fn terminal_parameter_reports_are_queued_as_terminal_responses() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("\x1b[x\x1b[1x").unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[2;1;1;128;128;1;0x\x1b[3;1;1;128;128;1;0x"
    );
}

#[test]
fn ansi_mode_reports_return_insert_mode_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str("\x1b[4$p\x1b[4h\x1b[4$p\x1b[20$p\x1b[20h\x1b[20$p\x1b[999$p")
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[4;2$y\x1b[4;1$y\x1b[20;2$y\x1b[20;1$y\x1b[999;0$y"
    );
}

#[test]
fn dec_private_mode_reports_return_mode_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str(
            "\x1b[?7$p\x1b[?7l\x1b[?7$p\x1b[?66$p\x1b[?66h\x1b[?66$p\x1b[?2004h\x1b[?2004$p\x1b[?999$p",
        )
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[?7;1$y\x1b[?7;2$y\x1b[?66;2$y\x1b[?66;1$y\x1b[?2004;1$y\x1b[?999;0$y"
    );
}

#[test]
fn dec_private_mode_reports_include_alternate_screen_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str(
            "\x1b[?47$p\x1b[?1047$p\x1b[?1049$p\x1b[?1049h\x1b[?47$p\x1b[?1047$p\x1b[?1049$p\x1b[?1049l\x1b[?1049$p",
        )
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[?47;2$y\x1b[?1047;2$y\x1b[?1049;2$y\x1b[?47;1$y\x1b[?1047;1$y\x1b[?1049;1$y\x1b[?1049;2$y"
    );
}

#[test]
fn decrqss_reports_scroll_margin_status_string() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 5).unwrap());

    terminal.write_str("\x1b[2;4r\x1bP$qr\x1b\\").unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1bP1$r2;4r\x1b\\"
    );
}

#[test]
fn decrqss_reports_cursor_shape_status_string() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 5).unwrap());

    terminal.write_str("\x1b[6 q\x1bP$q q\x1b\\").unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1bP1$r6 q\x1b\\");
}

#[test]
fn decrqss_reports_default_sgr_status_string() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 5).unwrap());

    terminal.write_str("\x1bP$qm\x1b\\").unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1bP1$r0m\x1b\\");
}

#[test]
fn decrqss_reports_active_sgr_status_string() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 5).unwrap());

    terminal
        .write_str("\x1b[1;3;7;31;44m\x1bP$qm\x1b\\")
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1bP1$r1;3;7;31;44m\x1b\\"
    );
}

#[test]
fn decrqss_reports_sgr_underline_color_status_string() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 5).unwrap());

    terminal
        .write_str("\x1b[4;58:2:17:34:51m\x1bP$qm\x1b\\\x1b[59m\x1bP$qm\x1b\\")
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1bP1$r4;58:2:17:34:51m\x1b\\\x1bP1$r4m\x1b\\"
    );
}

#[test]
fn decrqss_rejects_unsupported_status_strings() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 5).unwrap());

    terminal.write_str("\x1bP$qz\x1b\\").unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1bP0$r\x1b\\");
}

#[test]
fn decrqss_rejects_oversized_status_strings_without_leaking_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 5).unwrap());
    let oversized = "z".repeat(65);

    terminal
        .write_str(&format!("\x1bP$q{oversized}\x1b\\\x1bP$qm\x1b\\"))
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1bP0$r\x1b\\\x1bP1$r0m\x1b\\"
    );
}

#[test]
fn text_area_size_report_uses_current_terminal_dimensions() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal.write_str("\x1b[18t").unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1b[8;5;12t");
}

#[test]
fn screen_size_report_uses_current_terminal_dimensions() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal.write_str("\x1b[19t").unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1b[9;5;12t");
}

#[test]
fn window_state_report_returns_open_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal.write_str("\x1b[11t").unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1b[1t");
}

#[test]
fn window_position_report_returns_origin_when_native_position_is_unknown() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal.write_str("\x1b[13t").unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1b[3;0;0t");
}

#[test]
fn pixel_window_size_report_uses_configured_pixel_dimensions() {
    let config = TerminalConfig::new(12, 5)
        .unwrap()
        .with_pixel_size(960, 540)
        .unwrap();
    let mut terminal = Terminal::new(config);

    terminal.write_str("\x1b[14t").unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1b[4;540;960t");
}

#[test]
fn primary_device_attributes_are_queued_as_terminal_responses() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("\x1b[c\x1b[0c").unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[?1;2c\x1b[?1;2c"
    );
}

#[test]
fn decid_queues_primary_device_attributes_response() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("\x1bZ").unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1b[?1;2c");
}

#[test]
fn c1_decid_queues_primary_device_attributes_response() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_bytes(b"\x9a").unwrap();

    assert_eq!(terminal.take_pending_response_bytes(), b"\x1b[?1;2c");
}

#[test]
fn secondary_device_attributes_are_queued_as_terminal_responses() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal.write_str("\x1b[>c\x1b[>0c").unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[>0;1;0c\x1b[>0;1;0c"
    );
}

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

#[test]
fn newline_at_bottom_moves_oldest_line_to_scrollback() {
    let config = TerminalConfig::new(8, 2)
        .unwrap()
        .with_scrollback_limit(4)
        .unwrap();
    let mut terminal = Terminal::new(config);

    terminal.write_str("one\r\ntwo\r\nthree").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "two");
    assert_eq!(grid.line_text(1), "three");
    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["one"]);
}

#[test]
fn long_output_keeps_scrollback_bounded_to_recent_lines() {
    let config = TerminalConfig::new(16, 4)
        .unwrap()
        .with_scrollback_limit(32)
        .unwrap();
    let mut terminal = Terminal::new(config);

    for index in 0..200 {
        terminal
            .write_str(&format!("gromaq-{index:03}\r\n"))
            .unwrap();
    }

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines.len(), 32);
    assert_eq!(scrollback.cells.len(), 32);
    assert_eq!(
        scrollback.lines.first().map(String::as_str),
        Some("gromaq-165")
    );
    assert_eq!(
        scrollback.lines.last().map(String::as_str),
        Some("gromaq-196")
    );
    assert!(!scrollback.lines.iter().any(|line| line == "gromaq-000"));
    assert_eq!(terminal.dump_grid().line_text(1), "gromaq-198");
    assert_eq!(terminal.dump_grid().line_text(2), "gromaq-199");
    assert!(terminal.dump_perf_metrics().scrolls > 32);
}

#[test]
fn scrollback_preserves_wide_cell_metadata_when_row_scrolls_offscreen() {
    let config = TerminalConfig::new(4, 2)
        .unwrap()
        .with_scrollback_limit(4)
        .unwrap();
    let mut terminal = Terminal::new(config);

    terminal.write_str("ab界\r\ncd\r\nef").unwrap();

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["ab界"]);
    assert_eq!(scrollback.cells.len(), 1);
    assert_eq!(scrollback.cells[0][2].text, "界");
    assert!(scrollback.cells[0][2].is_wide_leading);
    assert!(scrollback.cells[0][3].is_wide_trailing);
}

#[test]
fn scrollback_preserves_rich_cell_metadata_when_row_scrolls_offscreen() {
    let config = TerminalConfig::new(8, 2)
        .unwrap()
        .with_scrollback_limit(4)
        .unwrap();
    let mut terminal = Terminal::new(config);

    terminal
        .write_str(
            "\x1b]8;;https://gromaq.dev\x1b\\\x1b[31;4;58:2:17:34:51mabcdefgh\x1b[0m\x1b]8;;\x1b\\\r\nnext\r\nlast",
        )
        .unwrap();

    let scrollback = terminal.dump_scrollback();
    assert_eq!(scrollback.lines, vec!["abcdefgh"]);
    assert_eq!(scrollback.hyperlinks, vec!["https://gromaq.dev"]);
    assert_eq!(scrollback.underline_colors, vec![Color::Rgb(17, 34, 51)]);
    for cell in &scrollback.cells[0] {
        assert_eq!(cell.hyperlink_id, 1);
        assert_eq!(cell.style.foreground, Color::Ansi(1));
        assert!(cell.style.underline);
        assert_eq!(cell.style.underline_color_id, 1);
    }
}

#[test]
fn vertical_tab_and_form_feed_follow_linefeed_without_carriage_return() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 4).unwrap());

    terminal.write_bytes(b"A\x0bB\x0cC").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "A");
    assert_eq!(grid.line_text(1), " B");
    assert_eq!(grid.line_text(2), "  C");
    assert_eq!(terminal.dump_cursor().row, 2);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn csi_erase_display_mode_3_clears_scrollback_only() {
    let config = TerminalConfig::new(8, 2)
        .unwrap()
        .with_scrollback_limit(4)
        .unwrap();
    let mut terminal = Terminal::new(config);
    terminal.write_str("one\r\ntwo\r\nthree").unwrap();
    assert_eq!(terminal.dump_scrollback().lines, vec!["one"]);

    terminal.write_str("\x1b[3J").unwrap();

    assert!(terminal.dump_scrollback().lines.is_empty());
    assert_eq!(terminal.dump_grid().line_text(0), "two");
    assert_eq!(terminal.dump_grid().line_text(1), "three");
}

#[test]
fn ris_resets_terminal_state_to_initial_defaults() {
    let config = TerminalConfig::new(8, 3)
        .unwrap()
        .with_scrollback_limit(4)
        .unwrap();
    let mut terminal = Terminal::new(config);

    terminal.write_str("old\r\nscroll\r\nstate\r\n").unwrap();
    assert!(!terminal.dump_scrollback().lines.is_empty());

    terminal
        .write_str("\x1b[31;1;7mA\x1b[?25l\x1b[?1000h\x1b[?1006h\x1b[?2004h\x1b[2;3r")
        .unwrap();
    assert_eq!(
        terminal.encode_paste_text("paste"),
        b"\x1b[200~paste\x1b[201~"
    );
    assert!(
        terminal
            .encode_mouse_event(MouseEvent::new(
                MouseEventKind::Press,
                MouseButton::Left,
                0,
                0
            ))
            .is_some()
    );

    terminal.write_str("\x1bcZ\x1b[3;1H\nQ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(grid.line_text(2), "Q");
    assert_eq!(terminal.dump_scrollback().lines, vec!["Z"]);
    assert_eq!(grid.cell(2, 0).style.foreground, Color::Default);
    assert!(!grid.cell(2, 0).style.bold);
    assert!(!grid.cell(2, 0).style.inverse);
    assert_eq!(terminal.dump_cursor().row, 2);
    assert_eq!(terminal.dump_cursor().col, 1);
    assert!(terminal.dump_cursor().visible);
    assert_eq!(terminal.encode_paste_text("paste"), b"paste");
    assert_eq!(
        terminal.encode_mouse_event(MouseEvent::new(
            MouseEventKind::Press,
            MouseButton::Left,
            0,
            0
        )),
        None
    );
}

#[test]
fn decstr_soft_reset_restores_modes_without_clearing_screen() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str("xy\x1b[1;1H\x1b[31;1m\x1b[?25l\x1b(0\x1b[4h\x1b[!pq")
        .unwrap();

    let grid = terminal.dump_grid();
    let reset_cell = grid.cell(0, 0);
    assert_eq!(grid.line_text(0), "qy");
    assert_eq!(reset_cell.text, "q");
    assert_eq!(reset_cell.style.foreground, Color::Default);
    assert!(!reset_cell.style.bold);
    assert!(terminal.dump_cursor().visible);
}

#[test]
fn decstr_soft_reset_resets_dec_saved_cursor_to_home_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str("\x1b[2;3H\x1b7\x1b[!p\x1b[3;4H\x1b8Z")
        .unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn decstr_soft_reset_disables_autowrap() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());

    terminal.write_str("\x1b[!pabcdE").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "abcE");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn csi_erase_display_mode_0_clears_from_cursor_to_screen_end() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal
        .write_str("abcd\r\nefgh\r\nijkl\x1b[2;3H\x1b[J")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "abcd");
    assert_eq!(grid.line_text(1), "ef");
    assert_eq!(grid.line_text(2), "");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn csi_erase_display_mode_1_clears_from_screen_start_to_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal
        .write_str("abcd\r\nefgh\r\nijkl\x1b[2;3H\x1b[1J")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "");
    assert_eq!(grid.line_text(1), "   h");
    assert_eq!(grid.line_text(2), "ijkl");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn csi_erase_display_mode_2_clears_screen_without_moving_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal
        .write_str("abcd\r\nefgh\r\nijkl\x1b[2;3H\x1b[2JZ")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "");
    assert_eq!(grid.line_text(1), "  Z");
    assert_eq!(grid.line_text(2), "");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn resize_preserves_visible_text_and_clamps_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 2).unwrap());
    terminal.write_str("abc\r\ndef").unwrap();

    terminal.resize(4, 3).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "abc");
    assert_eq!(grid.line_text(1), "def");
    assert!(terminal.dump_cursor().row < 3);
    assert!(terminal.dump_cursor().col < 4);
}
