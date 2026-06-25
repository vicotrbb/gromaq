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
