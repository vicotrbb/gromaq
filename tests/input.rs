use gromaq::{KeyModifiers, TestKey, encode_keys, encode_winit_key};
use winit::keyboard::{Key, ModifiersState, NamedKey};

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
        encode_winit_key(&Key::Named(NamedKey::ArrowLeft), ModifiersState::empty()),
        Some(b"\x1b[D".to_vec())
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
        encode_winit_key(&Key::Named(NamedKey::Shift), ModifiersState::SHIFT),
        None
    );
}
