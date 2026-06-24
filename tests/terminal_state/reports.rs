use super::*;

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
            "\x1b[?7$p\x1b[?7l\x1b[?7$p\x1b[?66$p\x1b=\x1b[?66$p\x1b>\x1b[?66$p\x1b[?66h\x1b[?66$p\x1b[?2004h\x1b[?2004$p\x1b[?999$p",
        )
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[?7;1$y\x1b[?7;2$y\x1b[?66;2$y\x1b[?66;1$y\x1b[?66;2$y\x1b[?66;1$y\x1b[?2004;1$y\x1b[?999;0$y"
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
fn dec_private_mode_reports_include_cursor_state_modes() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str(
            "\x1b[?1$p\x1b[?6$p\x1b[?12$p\x1b[?25$p\
             \x1b[?1h\x1b[?6h\x1b[?12h\x1b[?25l\
             \x1b[?1$p\x1b[?6$p\x1b[?12$p\x1b[?25$p",
        )
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[?1;2$y\x1b[?6;2$y\x1b[?12;1$y\x1b[?25;1$y\
          \x1b[?1;1$y\x1b[?6;1$y\x1b[?12;1$y\x1b[?25;2$y"
    );
}

#[test]
fn dec_private_mode_reports_include_mouse_and_focus_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str(
            "\x1b[?1000$p\x1b[?1002$p\x1b[?1003$p\x1b[?1004$p\x1b[?1006$p\
             \x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1004h\x1b[?1006h\
             \x1b[?1000$p\x1b[?1002$p\x1b[?1003$p\x1b[?1004$p\x1b[?1006$p",
        )
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[?1000;2$y\x1b[?1002;2$y\x1b[?1003;2$y\x1b[?1004;2$y\x1b[?1006;2$y\
          \x1b[?1000;1$y\x1b[?1002;1$y\x1b[?1003;1$y\x1b[?1004;1$y\x1b[?1006;1$y"
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
