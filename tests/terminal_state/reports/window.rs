use super::*;

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
