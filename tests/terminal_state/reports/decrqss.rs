use super::*;

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
        .write_str("\x1b[1;3;7;51;31;44m\x1bP$qm\x1b\\\x1b[52m\x1bP$qm\x1b\\")
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1bP1$r1;3;7;51;31;44m\x1b\\\x1bP1$r1;3;7;52;31;44m\x1b\\"
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
