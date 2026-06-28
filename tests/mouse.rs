use gromaq::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind, Terminal, TerminalConfig};

#[test]
fn mouse_reporting_is_disabled_by_default() {
    let terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    let event = MouseEvent::new(MouseEventKind::Press, MouseButton::Left, 2, 1);

    assert_eq!(terminal.encode_mouse_event(event), None);
}

#[test]
fn sgr_mouse_press_and_release_are_encoded_when_enabled() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b[?1000h\x1b[?1006h").unwrap();

    let press = MouseEvent::new(MouseEventKind::Press, MouseButton::Left, 2, 1);
    let release = MouseEvent::new(MouseEventKind::Release, MouseButton::Left, 2, 1);

    assert_eq!(terminal.encode_mouse_event(press).unwrap(), b"\x1b[<0;3;2M");
    assert_eq!(
        terminal.encode_mouse_event(release).unwrap(),
        b"\x1b[<0;3;2m"
    );
}

#[test]
fn default_mouse_protocol_reports_press_and_release_when_sgr_is_disabled() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b[?1000h").unwrap();

    let press = MouseEvent::new(MouseEventKind::Press, MouseButton::Left, 2, 1);
    let release = MouseEvent::new(MouseEventKind::Release, MouseButton::Left, 2, 1);

    assert_eq!(terminal.encode_mouse_event(press).unwrap(), b"\x1b[M #\"");
    assert_eq!(terminal.encode_mouse_event(release).unwrap(), b"\x1b[M##\"");
}

#[test]
fn x10_mouse_mode_reports_button_presses_only() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b[?9h").unwrap();

    let press = MouseEvent::new(MouseEventKind::Press, MouseButton::Left, 2, 1);
    let release = MouseEvent::new(MouseEventKind::Release, MouseButton::Left, 2, 1);
    let drag = MouseEvent::new(MouseEventKind::Drag, MouseButton::Left, 2, 1);

    assert_eq!(terminal.encode_mouse_event(press).unwrap(), b"\x1b[M #\"");
    assert_eq!(terminal.encode_mouse_event(release), None);
    assert_eq!(terminal.encode_mouse_event(drag), None);

    terminal.write_str("\x1b[?9l").unwrap();

    assert_eq!(terminal.encode_mouse_event(press), None);
}

#[test]
fn disabling_mouse_reporting_stops_encoding_events() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b[?1000h\x1b[?1006h").unwrap();
    terminal.write_str("\x1b[?1000l").unwrap();

    let event = MouseEvent::new(MouseEventKind::Press, MouseButton::Left, 2, 1);

    assert_eq!(terminal.encode_mouse_event(event), None);
}

#[test]
fn sgr_mouse_wheel_events_use_xterm_button_codes() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b[?1000h\x1b[?1006h").unwrap();

    let event = MouseEvent::new(MouseEventKind::Press, MouseButton::WheelUp, 0, 0);

    assert_eq!(
        terminal.encode_mouse_event(event).unwrap(),
        b"\x1b[<64;1;1M"
    );
}

#[test]
fn default_mouse_protocol_reports_wheel_button_codes() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b[?1000h").unwrap();

    let event = MouseEvent::new(MouseEventKind::Press, MouseButton::WheelDown, 0, 0);

    assert_eq!(terminal.encode_mouse_event(event).unwrap(), b"\x1b[Ma!!");
}

#[test]
fn mouse_reports_include_keyboard_modifier_bits() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b[?1000h\x1b[?1006h").unwrap();

    let event = MouseEvent::new(MouseEventKind::Press, MouseButton::Left, 2, 1)
        .with_modifiers(KeyModifiers::SHIFT | KeyModifiers::ALT | KeyModifiers::CTRL);

    assert_eq!(
        terminal.encode_mouse_event(event).unwrap(),
        b"\x1b[<28;3;2M"
    );
}

#[test]
fn default_mouse_protocol_reports_keyboard_modifier_bits() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b[?1000h").unwrap();

    let press = MouseEvent::new(MouseEventKind::Press, MouseButton::Right, 2, 1)
        .with_modifiers(KeyModifiers::SHIFT | KeyModifiers::CTRL);
    let release = MouseEvent::new(MouseEventKind::Release, MouseButton::Right, 2, 1)
        .with_modifiers(KeyModifiers::SHIFT | KeyModifiers::CTRL);

    assert_eq!(terminal.encode_mouse_event(press).unwrap(), b"\x1b[M6#\"");
    assert_eq!(terminal.encode_mouse_event(release).unwrap(), b"\x1b[M7#\"");
}

#[test]
fn default_mouse_protocol_rejects_coordinates_outside_byte_encoding() {
    let mut terminal = Terminal::new(TerminalConfig::new(240, 3).unwrap());
    terminal.write_str("\x1b[?1000h").unwrap();

    let event = MouseEvent::new(MouseEventKind::Press, MouseButton::Left, 224, 0);

    assert_eq!(terminal.encode_mouse_event(event), None);
}

#[test]
fn sgr_mouse_protocol_rejects_coordinates_that_cannot_be_one_based() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b[?1000h\x1b[?1006h").unwrap();

    let event = MouseEvent::new(MouseEventKind::Press, MouseButton::Left, u16::MAX, 0);

    assert_eq!(terminal.encode_mouse_event(event), None);
}

#[test]
fn button_motion_mode_reports_drag_but_not_plain_motion() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b[?1002h\x1b[?1006h").unwrap();

    let drag = MouseEvent::new(MouseEventKind::Drag, MouseButton::Left, 4, 2);
    let motion = MouseEvent::new(MouseEventKind::Motion, MouseButton::Left, 4, 2);

    assert_eq!(terminal.encode_mouse_event(drag).unwrap(), b"\x1b[<32;5;3M");
    assert_eq!(terminal.encode_mouse_event(motion), None);
}

#[test]
fn any_motion_mode_reports_motion_without_button_press() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b[?1003h\x1b[?1006h").unwrap();

    let motion = MouseEvent::new(MouseEventKind::Motion, MouseButton::None, 6, 1);

    assert_eq!(
        terminal.encode_mouse_event(motion).unwrap(),
        b"\x1b[<35;7;2M"
    );
}

#[test]
fn disabling_motion_modes_keeps_basic_button_reporting_when_1000_remains_enabled() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal
        .write_str("\x1b[?1000h\x1b[?1002h\x1b[?1006h")
        .unwrap();
    terminal.write_str("\x1b[?1002l").unwrap();

    let drag = MouseEvent::new(MouseEventKind::Drag, MouseButton::Left, 1, 1);
    let press = MouseEvent::new(MouseEventKind::Press, MouseButton::Left, 1, 1);

    assert_eq!(terminal.encode_mouse_event(drag), None);
    assert_eq!(terminal.encode_mouse_event(press).unwrap(), b"\x1b[<0;2;2M");
}

#[test]
fn disabling_motion_mode_without_button_reporting_disables_mouse_events() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal.write_str("\x1b[?1002h\x1b[?1006h").unwrap();

    let drag = MouseEvent::new(MouseEventKind::Drag, MouseButton::Left, 1, 1);
    let press = MouseEvent::new(MouseEventKind::Press, MouseButton::Left, 1, 1);

    assert_eq!(terminal.encode_mouse_event(drag).unwrap(), b"\x1b[<32;2;2M");

    terminal.write_str("\x1b[?1002l").unwrap();

    assert_eq!(terminal.encode_mouse_event(drag), None);
    assert_eq!(terminal.encode_mouse_event(press), None);
}

#[test]
fn disabling_button_reporting_preserves_active_motion_reporting() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal
        .write_str("\x1b[?1000h\x1b[?1002h\x1b[?1006h")
        .unwrap();

    terminal.write_str("\x1b[?1000l").unwrap();

    let drag = MouseEvent::new(MouseEventKind::Drag, MouseButton::Left, 1, 1);
    let press = MouseEvent::new(MouseEventKind::Press, MouseButton::Left, 1, 1);

    assert_eq!(terminal.encode_mouse_event(drag).unwrap(), b"\x1b[<32;2;2M");
    assert_eq!(terminal.encode_mouse_event(press).unwrap(), b"\x1b[<0;2;2M");
}

#[test]
fn any_motion_reporting_survives_button_motion_reset() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal
        .write_str("\x1b[?1002h\x1b[?1003h\x1b[?1006h")
        .unwrap();

    terminal.write_str("\x1b[?1002l").unwrap();

    let motion = MouseEvent::new(MouseEventKind::Motion, MouseButton::None, 1, 1);

    assert_eq!(
        terminal.encode_mouse_event(motion).unwrap(),
        b"\x1b[<35;2;2M"
    );
}

#[test]
fn dec_private_restore_restores_button_reporting_and_sgr_protocol() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal
        .write_str("\x1b[?1000h\x1b[?1006h\x1b[?1000s\x1b[?1006s")
        .unwrap();
    terminal.write_str("\x1b[?1000l\x1b[?1006l").unwrap();

    let press = MouseEvent::new(MouseEventKind::Press, MouseButton::Left, 1, 1);
    assert_eq!(terminal.encode_mouse_event(press), None);

    terminal.write_str("\x1b[?1000r\x1b[?1006r").unwrap();

    assert_eq!(terminal.encode_mouse_event(press).unwrap(), b"\x1b[<0;2;2M");
}

#[test]
fn dec_private_restore_keeps_button_motion_and_any_motion_independent() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());
    terminal
        .write_str("\x1b[?1002h\x1b[?1002s\x1b[?1003s\x1b[?1006h")
        .unwrap();
    terminal.write_str("\x1b[?1002l\x1b[?1003h").unwrap();

    let drag = MouseEvent::new(MouseEventKind::Drag, MouseButton::Left, 1, 1);
    let motion = MouseEvent::new(MouseEventKind::Motion, MouseButton::None, 1, 1);
    assert_eq!(
        terminal.encode_mouse_event(motion).unwrap(),
        b"\x1b[<35;2;2M"
    );

    terminal.write_str("\x1b[?1002r\x1b[?1003r").unwrap();

    assert_eq!(terminal.encode_mouse_event(drag).unwrap(), b"\x1b[<32;2;2M");
    assert_eq!(terminal.encode_mouse_event(motion), None);
}
