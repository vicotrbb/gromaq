use base64::{Engine, engine::general_purpose};
use gromaq::{Terminal, TerminalConfig};

const MAX_OSC52_CLIPBOARD_BYTES: usize = 1_048_576;
const MAX_OSC_TITLE_BYTES: usize = 4096;
const MAX_METADATA_IDS: u16 = 4096;

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
fn csi_window_title_stack_restores_saved_icon_label_and_title() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str(
            "\x1b]0;Initial\x07\
             \x1b[22;0t\
             \x1b]1;Changed Icon\x07\
             \x1b]2;Changed Title\x07\
             \x1b[23;0t\
             \x1b[20t\
             \x1b[21t",
        )
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b]LInitial\x1b\\\x1b]lInitial\x1b\\"
    );
}

#[test]
fn overlong_osc_title_is_ignored_without_clearing_previous_title() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    let overlong_title = "x".repeat(MAX_OSC_TITLE_BYTES + 1);

    terminal.write_str("\x1b]2;safe title\x07").unwrap();
    terminal
        .write_str(&format!("\x1b]2;{overlong_title}\x07\x1b[21t"))
        .unwrap();

    assert_eq!(terminal.dump_title().as_deref(), Some("safe title"));
    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b]lsafe title\x1b\\"
    );
}

#[test]
fn overlong_osc_icon_label_is_ignored_without_clearing_previous_label() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    let overlong_label = "x".repeat(MAX_OSC_TITLE_BYTES + 1);

    terminal.write_str("\x1b]0;safe label\x07").unwrap();
    terminal
        .write_str(&format!("\x1b]1;{overlong_label}\x07\x1b[20t"))
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b]Lsafe label\x1b\\"
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
fn osc_8_hyperlink_table_is_bounded_without_panicking() {
    let mut terminal = Terminal::new(TerminalConfig::new(MAX_METADATA_IDS + 2, 1).unwrap());
    let mut input = String::new();

    for index in 0..=MAX_METADATA_IDS {
        input.push_str(&format!("\x1b]8;;https://gromaq.dev/{index}\x1b\\x"));
    }

    terminal.write_str(&input).unwrap();

    let grid = terminal.dump_grid();
    assert_eq!(grid.cell_hyperlink(0, 0), Some("https://gromaq.dev/0"));
    assert_eq!(
        grid.cell_hyperlink(0, MAX_METADATA_IDS - 1),
        Some("https://gromaq.dev/4095")
    );
    assert_eq!(grid.cell_hyperlink(0, MAX_METADATA_IDS), None);
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
fn oversized_osc_52_encoded_payload_is_ignored_without_side_effects() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b]52;c;SGVsbG8=\x07").unwrap();
    let max_encoded_len = MAX_OSC52_CLIPBOARD_BYTES.div_ceil(3) * 4;
    let payload = "A".repeat(max_encoded_len + 4);

    terminal
        .write_str(&format!("\x1b]52;c;{payload}\x07"))
        .unwrap();

    assert_eq!(terminal.dump_clipboard_text().as_deref(), Some("Hello"));
}

#[test]
fn oversized_osc_52_decoded_payload_is_ignored_without_side_effects() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b]52;c;SGVsbG8=\x07").unwrap();
    let payload = general_purpose::STANDARD.encode(vec![b'x'; MAX_OSC52_CLIPBOARD_BYTES + 1]);

    terminal
        .write_str(&format!("\x1b]52;c;{payload}\x07"))
        .unwrap();

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
fn bracketed_paste_wraps_multiline_utf8_payloads_without_reencoding() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    let text = "alpha\nbeta\t界";

    terminal.write_str("\x1b[?2004h").unwrap();

    assert_eq!(
        terminal.encode_paste_text(text),
        b"\x1b[200~alpha\nbeta\t\xe7\x95\x8c\x1b[201~"
    );
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
