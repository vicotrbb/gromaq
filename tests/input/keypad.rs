use gromaq::{Terminal, TerminalConfig};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

#[test]
fn terminal_encodes_physical_numpad_keys_in_numeric_mode() {
    let terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Character("1".into()),
            Some(PhysicalKey::Code(KeyCode::Numpad1)),
            ModifiersState::empty(),
        ),
        Some(b"1".to_vec())
    );
    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Named(NamedKey::Enter),
            Some(PhysicalKey::Code(KeyCode::NumpadEnter)),
            ModifiersState::empty(),
        ),
        Some(b"\r".to_vec())
    );
    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Named(NamedKey::Enter),
            Some(PhysicalKey::Code(KeyCode::NumpadEnter)),
            ModifiersState::ALT,
        ),
        Some(b"\x1b\r".to_vec())
    );
    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Character("+".into()),
            Some(PhysicalKey::Code(KeyCode::NumpadAdd)),
            ModifiersState::ALT,
        ),
        Some(b"\x1b+".to_vec())
    );
}

#[test]
fn terminal_encodes_physical_numpad_keys_in_application_keypad_mode() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    terminal.write_str("\x1b[?66h").unwrap();

    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Character("1".into()),
            Some(PhysicalKey::Code(KeyCode::Numpad1)),
            ModifiersState::empty(),
        ),
        Some(b"\x1bOq".to_vec())
    );
    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Named(NamedKey::Enter),
            Some(PhysicalKey::Code(KeyCode::NumpadEnter)),
            ModifiersState::empty(),
        ),
        Some(b"\x1bOM".to_vec())
    );
    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Character("1".into()),
            Some(PhysicalKey::Code(KeyCode::Numpad1)),
            ModifiersState::ALT,
        ),
        Some(b"\x1b\x1bOq".to_vec())
    );
    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Named(NamedKey::Enter),
            Some(PhysicalKey::Code(KeyCode::NumpadEnter)),
            ModifiersState::ALT,
        ),
        Some(b"\x1b\x1bOM".to_vec())
    );
    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Character("1".into()),
            None,
            ModifiersState::empty(),
        ),
        Some(b"1".to_vec())
    );

    terminal.write_str("\x1b[?66l").unwrap();
    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Character("1".into()),
            Some(PhysicalKey::Code(KeyCode::Numpad1)),
            ModifiersState::empty(),
        ),
        Some(b"1".to_vec())
    );
}

#[test]
fn terminal_encodes_physical_numpad_keys_after_decpam_decpnm() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b=").unwrap();
    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Character("1".into()),
            Some(PhysicalKey::Code(KeyCode::Numpad1)),
            ModifiersState::empty(),
        ),
        Some(b"\x1bOq".to_vec())
    );
    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Named(NamedKey::Enter),
            Some(PhysicalKey::Code(KeyCode::NumpadEnter)),
            ModifiersState::empty(),
        ),
        Some(b"\x1bOM".to_vec())
    );

    terminal.write_str("\x1b>").unwrap();
    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Character("1".into()),
            Some(PhysicalKey::Code(KeyCode::Numpad1)),
            ModifiersState::empty(),
        ),
        Some(b"1".to_vec())
    );
}

#[test]
fn terminal_restores_saved_application_keypad_mode() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b[?66h\x1b[?66s\x1b[?66l").unwrap();
    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Character("1".into()),
            Some(PhysicalKey::Code(KeyCode::Numpad1)),
            ModifiersState::empty(),
        ),
        Some(b"1".to_vec())
    );

    terminal.write_str("\x1b[?66r").unwrap();
    assert_eq!(
        terminal.encode_winit_key_event_input(
            &Key::Character("1".into()),
            Some(PhysicalKey::Code(KeyCode::Numpad1)),
            ModifiersState::empty(),
        ),
        Some(b"\x1bOq".to_vec())
    );
}
