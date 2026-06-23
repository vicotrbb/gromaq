use gromaq::{Terminal, TerminalConfig};

#[test]
fn osc_2_sets_window_title_with_bel_terminator() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal.write_str("\x1b]2;Gromaq Terminal\x07").unwrap();

    assert_eq!(terminal.dump_title().as_deref(), Some("Gromaq Terminal"));
}

#[test]
fn osc_0_sets_window_title_with_st_terminator() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal.write_str("\x1b]0;Icon and Title\x1b\\").unwrap();

    assert_eq!(terminal.dump_title().as_deref(), Some("Icon and Title"));
}

#[test]
fn osc_1_sets_icon_label_without_changing_window_title() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("\x1b]2;Window Title\x07\x1b]1;Icon Label\x07\x1b[20t\x1b[21t")
        .unwrap();

    assert_eq!(terminal.dump_title().as_deref(), Some("Window Title"));
    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b]LIcon Label\x1b\\\x1b]lWindow Title\x1b\\"
    );
}

#[test]
fn csi_window_title_report_returns_current_title() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("\x1b]2;Gromaq Terminal\x07\x1b[21t")
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b]lGromaq Terminal\x1b\\"
    );
}

#[test]
fn csi_window_icon_label_report_returns_current_title_label() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("\x1b]0;Gromaq Terminal\x07\x1b[20t")
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b]LGromaq Terminal\x1b\\"
    );
}

#[test]
fn osc_52_decodes_clipboard_text_without_changing_title() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b]2;safe title\x07").unwrap();

    terminal.write_str("\x1b]52;c;SGVsbG8=\x07").unwrap();

    assert_eq!(terminal.dump_title().as_deref(), Some("safe title"));
    assert_eq!(terminal.dump_clipboard_text().as_deref(), Some("Hello"));
}

#[test]
fn osc_8_hyperlink_applies_uri_to_printed_cells_until_reset() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("\x1b]8;;https://gromaq.dev\x1b\\hi\x1b]8;;\x1b\\!")
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "hi!");
    assert_eq!(grid.cell_hyperlink(0, 0), Some("https://gromaq.dev"));
    assert_eq!(grid.cell_hyperlink(0, 1), Some("https://gromaq.dev"));
    assert_eq!(grid.cell_hyperlink(0, 2), None);
}

#[test]
fn invalid_osc_8_hyperlink_uri_is_ignored_without_clearing_active_link() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    let overlong_uri = "x".repeat(4097);

    terminal
        .write_str(&format!(
            "\x1b]8;;https://gromaq.dev\x1b\\A\x1b]8;;{overlong_uri}\x1b\\B\x1b]8;;\x1b\\C"
        ))
        .unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.line_text(0), "ABC");
    assert_eq!(grid.cell_hyperlink(0, 0), Some("https://gromaq.dev"));
    assert_eq!(grid.cell_hyperlink(0, 1), Some("https://gromaq.dev"));
    assert_eq!(grid.cell_hyperlink(0, 2), None);
}

#[test]
fn invalid_osc_52_payload_is_ignored_without_side_effects() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b]52;c;SGVsbG8=\x07").unwrap();

    terminal.write_str("\x1b]52;c;not base64!!\x07").unwrap();
    terminal.write_str("\x1b]52;c;?\x07").unwrap();

    assert_eq!(terminal.dump_clipboard_text().as_deref(), Some("Hello"));
}

#[test]
fn paste_encoding_is_plain_text_until_bracketed_paste_is_enabled() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    assert_eq!(terminal.encode_paste_text("abc"), b"abc");

    terminal.write_str("\x1b[?2004h").unwrap();
    assert_eq!(terminal.encode_paste_text("abc"), b"\x1b[200~abc\x1b[201~");

    terminal.write_str("\x1b[?2004l").unwrap();
    assert_eq!(terminal.encode_paste_text("abc"), b"abc");
}

#[test]
fn dec_private_mode_restore_restores_saved_bracketed_paste_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str("\x1b[?2004h\x1b[?2004s\x1b[?2004l")
        .unwrap();
    assert_eq!(terminal.encode_paste_text("abc"), b"abc");

    terminal.write_str("\x1b[?2004r").unwrap();
    assert_eq!(terminal.encode_paste_text("abc"), b"\x1b[200~abc\x1b[201~");
}
