use gromaq::{KeyModifiers, Terminal, TerminalConfig, TestKey, encode_keys, encode_winit_key};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

#[test]
fn encodes_common_terminal_keys_to_bytes() {
    let keys = [
        TestKey::Char('a'),
        TestKey::Enter,
        TestKey::Backspace,
        TestKey::ArrowUp,
        TestKey::ArrowRight,
    ];

    assert_eq!(encode_keys(&keys), b"a\r\x7f\x1b[A\x1b[C");
}

#[test]
fn encodes_control_modified_ascii_characters() {
    let keys = [TestKey::ModifiedChar {
        ch: 'c',
        modifiers: KeyModifiers::CTRL,
    }];

    assert_eq!(encode_keys(&keys), vec![0x03]);
}

#[test]
fn encodes_control_modified_ascii_punctuation() {
    let cases = [
        (' ', 0x00),
        ('@', 0x00),
        ('2', 0x00),
        ('[', 0x1b),
        ('3', 0x1b),
        (']', 0x1d),
        ('5', 0x1d),
        ('\\', 0x1c),
        ('4', 0x1c),
        ('^', 0x1e),
        ('6', 0x1e),
        ('_', 0x1f),
        ('/', 0x1f),
        ('7', 0x1f),
        ('?', 0x7f),
        ('8', 0x7f),
    ];

    for (ch, expected) in cases {
        let keys = [TestKey::ModifiedChar {
            ch,
            modifiers: KeyModifiers::CTRL,
        }];
        assert_eq!(encode_keys(&keys), vec![expected], "Ctrl+{ch}");
    }
}

#[test]
fn encodes_winit_printable_and_named_keys() {
    assert_eq!(
        encode_winit_key(&Key::Character("x".into()), ModifiersState::empty()),
        Some(b"x".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()),
        Some(b"\r".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::Space), ModifiersState::empty()),
        Some(b" ".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::Tab), ModifiersState::SHIFT),
        Some(b"\x1b[Z".to_vec())
    );
    assert_eq!(
        encode_winit_key(
            &Key::Named(NamedKey::Tab),
            ModifiersState::SHIFT | ModifiersState::CONTROL,
        ),
        Some(b"\x1b[1;6Z".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::ArrowLeft), ModifiersState::empty()),
        Some(b"\x1b[D".to_vec())
    );
}

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

#[test]
fn encodes_winit_navigation_keys_to_terminal_sequences() {
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::Home), ModifiersState::empty()),
        Some(b"\x1b[H".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::End), ModifiersState::empty()),
        Some(b"\x1b[F".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::Insert), ModifiersState::empty()),
        Some(b"\x1b[2~".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::Delete), ModifiersState::empty()),
        Some(b"\x1b[3~".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::PageUp), ModifiersState::empty()),
        Some(b"\x1b[5~".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::PageDown), ModifiersState::empty()),
        Some(b"\x1b[6~".to_vec())
    );
}

#[test]
fn encodes_winit_function_keys_to_terminal_sequences() {
    let cases = [
        (NamedKey::F1, b"\x1bOP".as_slice()),
        (NamedKey::F2, b"\x1bOQ".as_slice()),
        (NamedKey::F3, b"\x1bOR".as_slice()),
        (NamedKey::F4, b"\x1bOS".as_slice()),
        (NamedKey::F5, b"\x1b[15~".as_slice()),
        (NamedKey::F6, b"\x1b[17~".as_slice()),
        (NamedKey::F7, b"\x1b[18~".as_slice()),
        (NamedKey::F8, b"\x1b[19~".as_slice()),
        (NamedKey::F9, b"\x1b[20~".as_slice()),
        (NamedKey::F10, b"\x1b[21~".as_slice()),
        (NamedKey::F11, b"\x1b[23~".as_slice()),
        (NamedKey::F12, b"\x1b[24~".as_slice()),
    ];

    for (key, expected) in cases {
        assert_eq!(
            encode_winit_key(&Key::Named(key), ModifiersState::empty()),
            Some(expected.to_vec())
        );
    }
}

#[test]
fn encodes_winit_extended_function_keys_to_shifted_terminal_sequences() {
    let cases = [
        (NamedKey::F13, b"\x1b[1;2P".as_slice()),
        (NamedKey::F14, b"\x1b[1;2Q".as_slice()),
        (NamedKey::F15, b"\x1b[1;2R".as_slice()),
        (NamedKey::F16, b"\x1b[1;2S".as_slice()),
        (NamedKey::F17, b"\x1b[15;2~".as_slice()),
        (NamedKey::F18, b"\x1b[17;2~".as_slice()),
        (NamedKey::F19, b"\x1b[18;2~".as_slice()),
        (NamedKey::F20, b"\x1b[19;2~".as_slice()),
        (NamedKey::F21, b"\x1b[20;2~".as_slice()),
        (NamedKey::F22, b"\x1b[21;2~".as_slice()),
        (NamedKey::F23, b"\x1b[23;2~".as_slice()),
        (NamedKey::F24, b"\x1b[24;2~".as_slice()),
    ];

    for (key, expected) in cases {
        assert_eq!(
            encode_winit_key(&Key::Named(key), ModifiersState::empty()),
            Some(expected.to_vec())
        );
    }
}

#[test]
fn encodes_winit_modified_named_keys_to_terminal_sequences() {
    let cases = [
        (
            NamedKey::ArrowUp,
            ModifiersState::SHIFT,
            b"\x1b[1;2A".as_slice(),
        ),
        (
            NamedKey::ArrowRight,
            ModifiersState::ALT,
            b"\x1b[1;3C".as_slice(),
        ),
        (
            NamedKey::ArrowLeft,
            ModifiersState::CONTROL,
            b"\x1b[1;5D".as_slice(),
        ),
        (
            NamedKey::End,
            ModifiersState::SHIFT.union(ModifiersState::CONTROL),
            b"\x1b[1;6F".as_slice(),
        ),
        (
            NamedKey::Delete,
            ModifiersState::ALT,
            b"\x1b[3;3~".as_slice(),
        ),
        (
            NamedKey::PageDown,
            ModifiersState::CONTROL,
            b"\x1b[6;5~".as_slice(),
        ),
        (NamedKey::F1, ModifiersState::SHIFT, b"\x1b[1;2P".as_slice()),
        (
            NamedKey::F5,
            ModifiersState::CONTROL,
            b"\x1b[15;5~".as_slice(),
        ),
        (
            NamedKey::F12,
            ModifiersState::SHIFT.union(ModifiersState::ALT),
            b"\x1b[24;4~".as_slice(),
        ),
        (
            NamedKey::F13,
            ModifiersState::CONTROL,
            b"\x1b[1;6P".as_slice(),
        ),
        (NamedKey::F24, ModifiersState::ALT, b"\x1b[24;4~".as_slice()),
    ];

    for (key, modifiers, expected) in cases {
        assert_eq!(
            encode_winit_key(&Key::Named(key), modifiers),
            Some(expected.to_vec())
        );
    }
}

#[test]
fn encodes_winit_modified_terminal_characters() {
    assert_eq!(
        encode_winit_key(&Key::Character("c".into()), ModifiersState::CONTROL),
        Some(vec![0x03])
    );
    assert_eq!(
        encode_winit_key(&Key::Character("x".into()), ModifiersState::ALT),
        Some(b"\x1bx".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Character("xy".into()), ModifiersState::ALT),
        Some(b"\x1bxy".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::Space), ModifiersState::ALT),
        Some(b"\x1b ".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::Space), ModifiersState::CONTROL),
        Some(vec![0x00])
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::Enter), ModifiersState::ALT),
        Some(b"\x1b\r".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::Backspace), ModifiersState::ALT),
        Some(b"\x1b\x7f".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::Escape), ModifiersState::ALT),
        Some(b"\x1b\x1b".to_vec())
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::Shift), ModifiersState::SHIFT),
        None
    );
}

#[test]
fn super_modified_keys_are_reserved_for_native_shortcuts() {
    assert_eq!(
        encode_winit_key(&Key::Character("l".into()), ModifiersState::SUPER),
        None
    );
    assert_eq!(
        encode_winit_key(&Key::Named(NamedKey::ArrowLeft), ModifiersState::SUPER),
        None
    );
    assert_eq!(
        encode_winit_key(
            &Key::Named(NamedKey::PageDown),
            ModifiersState::SUPER | ModifiersState::SHIFT
        ),
        None
    );
}
