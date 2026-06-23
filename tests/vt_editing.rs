use gromaq::{CursorShape, Terminal, TerminalConfig};

#[test]
fn horizontal_tab_advances_to_next_default_tab_stop() {
    let mut terminal = Terminal::new(TerminalConfig::new(16, 2).unwrap());

    terminal.write_str("a\tb").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "a       b");
    assert_eq!(terminal.dump_cursor().col, 9);
}

#[test]
fn escape_horizontal_tab_set_adds_custom_tab_stop() {
    let mut terminal = Terminal::new(TerminalConfig::new(16, 2).unwrap());

    terminal.write_str("\x1b[1;6H\x1bH\x1b[1;1H\tZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "     Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 6);
}

#[test]
fn csi_tab_clear_removes_current_default_tab_stop() {
    let mut terminal = Terminal::new(TerminalConfig::new(16, 2).unwrap());

    terminal.write_str("\x1b[1;9H\x1b[g\x1b[1;1H\tZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "               Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 15);
}

#[test]
fn csi_tab_clear_all_removes_default_tab_stops() {
    let mut terminal = Terminal::new(TerminalConfig::new(16, 2).unwrap());

    terminal.write_str("\x1b[3g\x1b[1;1H\tZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "               Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 15);
}

#[test]
fn csi_cursor_forward_tab_moves_across_default_tab_stops() {
    let mut terminal = Terminal::new(TerminalConfig::new(20, 2).unwrap());

    terminal.write_str("abc\x1b[2IZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "abc             Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 17);
}

#[test]
fn csi_cursor_backward_tab_moves_across_default_tab_stops() {
    let mut terminal = Terminal::new(TerminalConfig::new(20, 2).unwrap());

    terminal.write_str("\x1b[1;18H\x1b[2ZZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "        Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 9);
}

#[test]
fn csi_tab_navigation_clamps_to_viewport_edges() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 2).unwrap());

    terminal.write_str("\x1b[1;9H\x1b[4IZ").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "         Z");
    assert_eq!(terminal.dump_cursor().col, 9);

    terminal.write_str("\x1b[4ZZ").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "Z        Z");
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn dec_special_graphics_charset_maps_box_drawing_and_restores_ascii() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str("\x1b(0lqk\r\nx x\r\nmqj\x1b(Bq")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "┌─┐");
    assert_eq!(grid.line_text(1), "│ │");
    assert_eq!(grid.line_text(2), "└─┘q");
}

#[test]
fn shift_out_invokes_g1_dec_special_graphics_until_shift_in() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b)0\x0elqk\x0fq").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "┌─┐q");
}

#[test]
fn decaln_fills_complete_viewport_with_alignment_pattern() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 3).unwrap());

    terminal
        .write_str("\x1b[2;2r\x1b[31mABCD\r\n界Z\r\nxy\x1b#8")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "EEEE");
    assert_eq!(grid.line_text(1), "EEEE");
    assert_eq!(grid.line_text(2), "EEEE");
}

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
fn csi_erase_characters_blanks_cells_without_shifting_line() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 2).unwrap());

    terminal.write_str("abcdef\x1b[1;3H\x1b[2X").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "ab  ef");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 2);
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
fn csi_repeat_preceding_character_defaults_to_one_and_ignores_empty_history() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 2).unwrap());

    terminal.write_str("\x1b[bA\x1b[b").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "AA");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 2);
}

#[test]
fn csi_insert_and_delete_lines_affect_rows_below_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 4).unwrap());

    terminal.write_str("one\ntwo\nthree\nfour").unwrap();
    terminal.write_str("\x1b[2;1H\x1b[Linserted").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "one");
    assert_eq!(terminal.dump_grid().line_text(1), "inserted");
    assert_eq!(terminal.dump_grid().line_text(2), "two");
    assert_eq!(terminal.dump_grid().line_text(3), "three");

    terminal.write_str("\x1b[3;1H\x1b[M").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "one");
    assert_eq!(terminal.dump_grid().line_text(1), "inserted");
    assert_eq!(terminal.dump_grid().line_text(2), "three");
    assert_eq!(terminal.dump_grid().line_text(3), "");
}

#[test]
fn csi_insert_lines_respects_scroll_region_bottom() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[3;1H\x1b[L").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "one");
    assert_eq!(grid.line_text(2), "");
    assert_eq!(grid.line_text(3), "two");
    assert_eq!(grid.line_text(4), "bottom");
}

#[test]
fn csi_delete_lines_respects_scroll_region_bottom() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[3;1H\x1b[M").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "one");
    assert_eq!(grid.line_text(2), "three");
    assert_eq!(grid.line_text(3), "");
    assert_eq!(grid.line_text(4), "bottom");
}

#[test]
fn dec_and_sco_save_restore_cursor_positions() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal.write_str("ab\x1b7cd\x1b8Z").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "abZd");

    terminal.write_str("\x1b[s\x1b[2;5H!\x1b[uQ").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "abZQ");
    assert_eq!(terminal.dump_grid().line_text(1), "    !");
}

#[test]
fn dec_save_restore_cursor_restores_rendition_attributes() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("\x1b[31;1m\x1b7\x1b[0mplain\x1b8Z")
        .unwrap();

    let grid = terminal.dump_grid();
    let restored = grid.cell(0, 0);
    assert_eq!(restored.text, "Z");
    assert_eq!(restored.style.foreground, gromaq::Color::Ansi(1));
    assert!(restored.style.bold);

    let plain = grid.cell(0, 1);
    assert_eq!(plain.text, "l");
    assert_eq!(plain.style.foreground, gromaq::Color::Default);
    assert!(!plain.style.bold);
}

#[test]
fn dec_private_1048_saves_and_restores_cursor_without_alternate_screen() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 4).unwrap());

    terminal
        .write_str("\x1b[2;4H\x1b[?1048h\x1b[3;8H!\x1b[?1048lZ")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(1), "   Z");
    assert_eq!(grid.line_text(2), "       !");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 4);
}

#[test]
fn csi_cursor_character_absolute_moves_within_current_row() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("abcdef\r\x1b[4GZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "abcZef");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 4);
}

#[test]
fn csi_vertical_position_absolute_moves_within_current_column() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 4).unwrap());

    terminal.write_str("ab\ncd\n\x1b[3G\x1b[1dZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "abZ");
    assert_eq!(terminal.dump_grid().line_text(1), "cd");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 3);
}

#[test]
fn csi_cursor_next_line_moves_down_and_returns_to_column_zero() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 4).unwrap());

    terminal.write_str("ab\x1b[2EZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(0), "ab");
    assert_eq!(terminal.dump_grid().line_text(2), "Z");
    assert_eq!(terminal.dump_cursor().row, 2);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn csi_cursor_previous_line_moves_up_and_returns_to_column_zero() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 4).unwrap());

    terminal.write_str("\x1b[4;5H!\x1b[2FZ").unwrap();

    assert_eq!(terminal.dump_grid().line_text(1), "Z");
    assert_eq!(terminal.dump_grid().line_text(3), "    !");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn csi_scroll_up_shifts_viewport_without_moving_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 4).unwrap());

    terminal.write_str("one\ntwo\nthree\nfour\x1b[2S").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "three");
    assert_eq!(grid.line_text(1), "four");
    assert_eq!(grid.line_text(2), "");
    assert_eq!(grid.line_text(3), "");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 4);
}

#[test]
fn csi_scroll_down_shifts_viewport_without_moving_cursor() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 4).unwrap());

    terminal.write_str("one\ntwo\nthree\nfour\x1b[2T").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(grid.line_text(2), "one");
    assert_eq!(grid.line_text(3), "two");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 4);
}

#[test]
fn csi_scroll_up_respects_scroll_region() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[2S").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "three");
    assert_eq!(grid.line_text(2), "");
    assert_eq!(grid.line_text(3), "");
    assert_eq!(grid.line_text(4), "bottom");
}

#[test]
fn csi_scroll_down_respects_scroll_region() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[2T").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(grid.line_text(2), "");
    assert_eq!(grid.line_text(3), "one");
    assert_eq!(grid.line_text(4), "bottom");
}

#[test]
fn decstbm_constrains_linefeed_scrolling_to_region() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[4;1H\n").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "two");
    assert_eq!(grid.line_text(2), "three");
    assert_eq!(grid.line_text(3), "");
    assert_eq!(grid.line_text(4), "bottom");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 0);
}

#[test]
fn dec_origin_mode_positions_cursor_relative_to_scroll_region() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[2;4r\x1b[?6h\x1b[1;1HZ\x1b[3;1HQ")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "");
    assert_eq!(grid.line_text(1), "Z");
    assert_eq!(grid.line_text(3), "Q");
    assert_eq!(grid.line_text(4), "");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn dec_origin_mode_clamps_cursor_to_scroll_region_bottom() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal.write_str("\x1b[2;4r\x1b[?6h\x1b[9;1HZ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(3), "Z");
    assert_eq!(grid.line_text(4), "");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn dec_origin_mode_disable_returns_cursor_addressing_to_viewport() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[2;4r\x1b[?6h\x1b[1;1HZ\x1b[?6lQ")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "Q");
    assert_eq!(grid.line_text(1), "Z");
    assert_eq!(terminal.dump_cursor().row, 0);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn escape_index_scrolls_region_without_carriage_return() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[4;4H\x1bDZ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "two");
    assert_eq!(grid.line_text(2), "three");
    assert_eq!(grid.line_text(3), "   Z");
    assert_eq!(grid.line_text(4), "bottom");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 4);
}

#[test]
fn escape_next_line_scrolls_region_and_returns_to_column_zero() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[4;4H\x1bEZ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "two");
    assert_eq!(grid.line_text(2), "three");
    assert_eq!(grid.line_text(3), "Z");
    assert_eq!(grid.line_text(4), "bottom");
    assert_eq!(terminal.dump_cursor().row, 3);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn reverse_index_at_scroll_top_scrolls_region_down() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[2;1H\x1bM").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "");
    assert_eq!(grid.line_text(2), "one");
    assert_eq!(grid.line_text(3), "two");
    assert_eq!(grid.line_text(4), "bottom");
    assert_eq!(terminal.dump_cursor().row, 1);
    assert_eq!(terminal.dump_cursor().col, 0);
}

#[test]
fn reverse_index_above_scroll_top_moves_cursor_up_without_scrolling() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 5).unwrap());

    terminal
        .write_str("\x1b[1;1Htop\x1b[2;1Hone\x1b[3;1Htwo\x1b[4;1Hthree\x1b[5;1Hbottom")
        .unwrap();
    terminal.write_str("\x1b[2;4r\x1b[4;1H\x1bMZ").unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "top");
    assert_eq!(grid.line_text(1), "one");
    assert_eq!(grid.line_text(2), "Zwo");
    assert_eq!(grid.line_text(3), "three");
    assert_eq!(grid.line_text(4), "bottom");
    assert_eq!(terminal.dump_cursor().row, 2);
    assert_eq!(terminal.dump_cursor().col, 1);
}

#[test]
fn dec_private_cursor_visibility_mode_toggles_cursor_snapshot() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    assert!(terminal.dump_cursor().visible);

    terminal.write_str("\x1b[?25l").unwrap();
    assert!(!terminal.dump_cursor().visible);

    terminal.write_str("\x1b[?25h").unwrap();
    assert!(terminal.dump_cursor().visible);
}

#[test]
fn decscusr_sets_cursor_shape_and_blinking_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b[3 q").unwrap();
    let blinking_underline = terminal.dump_cursor();
    assert_eq!(blinking_underline.shape, CursorShape::Underline);
    assert!(blinking_underline.blinking);

    terminal.write_str("\x1b[6 q").unwrap();
    let steady_bar = terminal.dump_cursor();
    assert_eq!(steady_bar.shape, CursorShape::Bar);
    assert!(!steady_bar.blinking);

    terminal.write_str("\x1b[0 q").unwrap();
    let default_block = terminal.dump_cursor();
    assert_eq!(default_block.shape, CursorShape::Block);
    assert!(default_block.blinking);
}
