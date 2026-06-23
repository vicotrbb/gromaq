use gromaq::{SelectionRange, Terminal, TerminalConfig};

#[test]
fn copy_selection_returns_text_from_single_visible_row() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("hello world").unwrap();

    terminal.set_selection(SelectionRange::new((0, 1), (0, 4)));

    assert_eq!(terminal.copy_selection().unwrap(), "ello");
}

#[test]
fn copy_selection_returns_single_visible_cell() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("hello").unwrap();

    terminal.set_selection(SelectionRange::new((0, 1), (0, 1)));

    assert_eq!(terminal.copy_selection().unwrap(), "e");
}

#[test]
fn copy_selection_spans_visible_rows_in_grid_order() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("alpha\r\nbravo\r\ncharlie").unwrap();

    terminal.set_selection(SelectionRange::new((0, 2), (2, 3)));

    assert_eq!(terminal.copy_selection().unwrap(), "pha\nbravo\nchar");
}

#[test]
fn copy_selection_uses_displayed_scrollback_view() {
    let mut terminal = Terminal::new(
        TerminalConfig::new(6, 3)
            .unwrap()
            .with_scrollback_limit(8)
            .unwrap(),
    );
    terminal.write_str("one\r\ntwo\r\nthree\r\nfour").unwrap();

    assert!(terminal.scroll_display_up(1));
    terminal.set_selection(SelectionRange::new((0, 0), (1, 2)));

    assert_eq!(terminal.copy_selection().unwrap(), "one\ntwo");
}

#[test]
fn copy_selection_omits_newline_across_soft_wrapped_rows() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 3).unwrap());
    terminal.write_str("helloworld").unwrap();

    terminal.set_selection(SelectionRange::new((0, 0), (1, 4)));

    assert_eq!(terminal.copy_selection().unwrap(), "helloworld");
}

#[test]
fn copy_selection_preserves_newline_at_hard_breaks() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 3).unwrap());
    terminal.write_str("hello\r\nworld").unwrap();

    terminal.set_selection(SelectionRange::new((0, 0), (1, 4)));

    assert_eq!(terminal.copy_selection().unwrap(), "hello\nworld");
}

#[test]
fn reversed_selection_is_normalized() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    terminal.write_str("abcdef").unwrap();

    terminal.set_selection(SelectionRange::new((0, 4), (0, 1)));

    assert_eq!(terminal.copy_selection().unwrap(), "bcde");
}

#[test]
fn copy_selection_preserves_wide_cell_text_once() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    terminal.write_str("a界b").unwrap();

    terminal.set_selection(SelectionRange::new((0, 0), (0, 3)));

    assert_eq!(terminal.copy_selection().unwrap(), "a界b");
}

#[test]
fn copy_selection_includes_wide_cell_when_starting_on_trailing_half() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    terminal.write_str("a界b").unwrap();

    terminal.set_selection(SelectionRange::new((0, 2), (0, 3)));

    assert_eq!(terminal.copy_selection().unwrap(), "界b");
}

#[test]
fn copy_selection_includes_single_wide_cell_from_trailing_half() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    terminal.write_str("a界b").unwrap();

    terminal.set_selection(SelectionRange::new((0, 2), (0, 2)));

    assert_eq!(terminal.copy_selection().unwrap(), "界");
}

#[test]
fn copy_selection_preserves_emoji_modifier_zwj_cluster_text_once() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    terminal.write_str("a👩🏽\u{200d}💻b").unwrap();

    terminal.set_selection(SelectionRange::new((0, 0), (0, 3)));

    assert_eq!(terminal.copy_selection().unwrap(), "a👩🏽\u{200d}💻b");
}

#[test]
fn copy_selection_preserves_rainbow_flag_zwj_cluster_text_once() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    terminal.write_str("a🏳️\u{200d}🌈b").unwrap();

    terminal.set_selection(SelectionRange::new((0, 0), (0, 3)));

    assert_eq!(terminal.copy_selection().unwrap(), "a🏳️\u{200d}🌈b");
}

#[test]
fn copy_selection_preserves_tag_sequence_emoji_flag_text_once() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    terminal
        .write_str("a🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}b")
        .unwrap();

    terminal.set_selection(SelectionRange::new((0, 0), (0, 3)));

    assert_eq!(
        terminal.copy_selection().unwrap(),
        "a🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}b"
    );
}

#[test]
fn copy_selection_preserves_selected_spaces_before_later_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    terminal.write_str("a  b").unwrap();

    terminal.set_selection(SelectionRange::new((0, 0), (0, 2)));

    assert_eq!(terminal.copy_selection().unwrap(), "a  ");
}

#[test]
fn clearing_selection_removes_copy_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());
    terminal.write_str("abcdef").unwrap();
    terminal.set_selection(SelectionRange::new((0, 0), (0, 2)));

    terminal.clear_selection();

    assert_eq!(terminal.copy_selection(), None);
}

#[test]
fn resizing_visible_grid_clears_stale_selection() {
    let mut terminal = Terminal::new(TerminalConfig::new(6, 2).unwrap());
    terminal.write_str("abcdef").unwrap();
    terminal.set_selection(SelectionRange::new((0, 1), (0, 3)));

    terminal.resize(4, 2).unwrap();

    assert_eq!(terminal.copy_selection(), None);
}

#[test]
fn alternate_screen_transitions_clear_visible_grid_selection() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    terminal.write_str("primary").unwrap();
    terminal.set_selection(SelectionRange::new((0, 0), (0, 2)));

    terminal.write_str("\x1b[?1049h").unwrap();

    assert_eq!(terminal.copy_selection(), None);

    terminal.write_str("alt").unwrap();
    terminal.set_selection(SelectionRange::new((0, 0), (0, 2)));
    terminal.write_str("\x1b[?1049l").unwrap();

    assert_eq!(terminal.copy_selection(), None);
}

#[test]
fn copy_selection_clamps_rows_below_visible_grid() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 2).unwrap());
    terminal.write_str("abcde\r\nvwxyz").unwrap();

    terminal.set_selection(SelectionRange::new((4, 1), (4, 3)));

    assert_eq!(terminal.copy_selection().unwrap(), "wxy");
}

#[test]
fn copy_selection_renormalizes_after_viewport_clamping() {
    let mut terminal = Terminal::new(TerminalConfig::new(5, 2).unwrap());
    terminal.write_str("abcde\r\nvwxyz").unwrap();

    terminal.set_selection(SelectionRange::new((5, 4), (6, 1)));

    assert_eq!(terminal.copy_selection().unwrap(), "wxyz");
}
