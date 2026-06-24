//! Numeric keypad input encoding.

use winit::keyboard::{KeyCode, PhysicalKey};

use super::{KeyModifiers, encode_alt_prefixed_key, encode_modified_char};

pub(super) fn encode_winit_keypad_key(
    physical_key: PhysicalKey,
    modifiers: KeyModifiers,
    application_keypad: bool,
) -> Option<Vec<u8>> {
    let PhysicalKey::Code(code) = physical_key else {
        return None;
    };

    if application_keypad {
        let final_byte = application_keypad_final_byte(code)?;
        return Some(encode_alt_prefixed_key(
            &[0x1b, b'O', final_byte],
            modifiers,
        ));
    }

    if code == KeyCode::NumpadEnter {
        return Some(encode_alt_prefixed_key(b"\r", modifiers));
    }
    let ch = numeric_keypad_char(code)?;
    let mut bytes = Vec::new();
    encode_modified_char(&mut bytes, ch, modifiers);
    Some(bytes)
}

fn application_keypad_final_byte(code: KeyCode) -> Option<u8> {
    match code {
        KeyCode::Numpad0 => Some(b'p'),
        KeyCode::Numpad1 => Some(b'q'),
        KeyCode::Numpad2 => Some(b'r'),
        KeyCode::Numpad3 => Some(b's'),
        KeyCode::Numpad4 => Some(b't'),
        KeyCode::Numpad5 => Some(b'u'),
        KeyCode::Numpad6 => Some(b'v'),
        KeyCode::Numpad7 => Some(b'w'),
        KeyCode::Numpad8 => Some(b'x'),
        KeyCode::Numpad9 => Some(b'y'),
        KeyCode::NumpadDecimal => Some(b'n'),
        KeyCode::NumpadDivide => Some(b'o'),
        KeyCode::NumpadMultiply => Some(b'j'),
        KeyCode::NumpadSubtract => Some(b'm'),
        KeyCode::NumpadAdd => Some(b'k'),
        KeyCode::NumpadEnter => Some(b'M'),
        KeyCode::NumpadEqual => Some(b'X'),
        _ => None,
    }
}

fn numeric_keypad_char(code: KeyCode) -> Option<char> {
    match code {
        KeyCode::Numpad0 => Some('0'),
        KeyCode::Numpad1 => Some('1'),
        KeyCode::Numpad2 => Some('2'),
        KeyCode::Numpad3 => Some('3'),
        KeyCode::Numpad4 => Some('4'),
        KeyCode::Numpad5 => Some('5'),
        KeyCode::Numpad6 => Some('6'),
        KeyCode::Numpad7 => Some('7'),
        KeyCode::Numpad8 => Some('8'),
        KeyCode::Numpad9 => Some('9'),
        KeyCode::NumpadDecimal => Some('.'),
        KeyCode::NumpadDivide => Some('/'),
        KeyCode::NumpadMultiply => Some('*'),
        KeyCode::NumpadSubtract => Some('-'),
        KeyCode::NumpadAdd => Some('+'),
        KeyCode::NumpadEqual => Some('='),
        _ => None,
    }
}
