//! Keyboard input encoding.

use bitflags::bitflags;
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

bitflags! {
    /// Keyboard modifier flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct KeyModifiers: u8 {
        /// Shift key.
        const SHIFT = 0b0000_0001;
        /// Control key.
        const CTRL = 0b0000_0010;
        /// Alt or option key.
        const ALT = 0b0000_0100;
        /// Super, command, or Windows key.
        const SUPER = 0b0000_1000;
    }
}

/// Deterministic key representation for tests and input mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestKey {
    /// Printable character.
    Char(char),
    /// Printable character with modifiers.
    ModifiedChar {
        /// Character to encode.
        ch: char,
        /// Active modifiers.
        modifiers: KeyModifiers,
    },
    /// Enter key.
    Enter,
    /// Backspace key.
    Backspace,
    /// Escape key.
    Escape,
    /// Up arrow.
    ArrowUp,
    /// Down arrow.
    ArrowDown,
    /// Left arrow.
    ArrowLeft,
    /// Right arrow.
    ArrowRight,
    /// Tab key.
    Tab,
}

/// Encode structured test keys into terminal input bytes.
pub fn encode_keys(keys: &[TestKey]) -> Vec<u8> {
    encode_keys_with_application_cursor_mode(keys, false)
}

/// Encode structured test keys with optional application cursor-key mode.
pub(crate) fn encode_keys_with_application_cursor_mode(
    keys: &[TestKey],
    application_cursor_keys: bool,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    for key in keys {
        match *key {
            TestKey::Char(ch) => push_char(&mut bytes, ch),
            TestKey::ModifiedChar { ch, modifiers } => {
                encode_modified_char(&mut bytes, ch, modifiers)
            }
            TestKey::Enter => bytes.push(b'\r'),
            TestKey::Backspace => bytes.push(0x7f),
            TestKey::Escape => bytes.push(0x1b),
            TestKey::ArrowUp if application_cursor_keys => bytes.extend_from_slice(b"\x1bOA"),
            TestKey::ArrowDown if application_cursor_keys => bytes.extend_from_slice(b"\x1bOB"),
            TestKey::ArrowRight if application_cursor_keys => bytes.extend_from_slice(b"\x1bOC"),
            TestKey::ArrowLeft if application_cursor_keys => bytes.extend_from_slice(b"\x1bOD"),
            TestKey::ArrowUp => bytes.extend_from_slice(b"\x1b[A"),
            TestKey::ArrowDown => bytes.extend_from_slice(b"\x1b[B"),
            TestKey::ArrowRight => bytes.extend_from_slice(b"\x1b[C"),
            TestKey::ArrowLeft => bytes.extend_from_slice(b"\x1b[D"),
            TestKey::Tab => bytes.push(b'\t'),
        }
    }
    bytes
}

/// Encode a `winit` logical key plus active modifiers into terminal input bytes.
pub fn encode_winit_key(key: &Key, modifiers: ModifiersState) -> Option<Vec<u8>> {
    encode_winit_key_with_application_cursor_mode(key, modifiers, false)
}

/// Encode a `winit` logical key with optional application cursor-key mode.
pub(crate) fn encode_winit_key_with_application_cursor_mode(
    key: &Key,
    modifiers: ModifiersState,
    application_cursor_keys: bool,
) -> Option<Vec<u8>> {
    encode_winit_key_with_terminal_modes(key, None, modifiers, application_cursor_keys, false)
}

/// Encode a `winit` key event according to terminal input modes.
pub(crate) fn encode_winit_key_with_terminal_modes(
    key: &Key,
    physical_key: Option<PhysicalKey>,
    modifiers: ModifiersState,
    application_cursor_keys: bool,
    application_keypad: bool,
) -> Option<Vec<u8>> {
    let key_modifiers = key_modifiers_from_winit(modifiers);
    if key_modifiers.contains(KeyModifiers::SUPER) {
        return None;
    }
    if let Some(physical_key) = physical_key
        && let Some(bytes) =
            encode_winit_keypad_key(physical_key, key_modifiers, application_keypad)
    {
        return Some(bytes);
    }

    let key = match key {
        Key::Character(text) => {
            let mut chars = text.chars();
            let ch = chars.next()?;
            if chars.next().is_some() {
                let mut bytes = Vec::new();
                for ch in text.chars() {
                    push_char(&mut bytes, ch);
                }
                return Some(bytes);
            }
            TestKey::ModifiedChar {
                ch,
                modifiers: key_modifiers,
            }
        }
        Key::Named(NamedKey::Enter) => return Some(encode_alt_prefixed_key(b"\r", key_modifiers)),
        Key::Named(NamedKey::Backspace) => {
            return Some(encode_alt_prefixed_key(b"\x7f", key_modifiers));
        }
        Key::Named(NamedKey::Escape) => {
            return Some(encode_alt_prefixed_key(b"\x1b", key_modifiers));
        }
        Key::Named(NamedKey::Space) => TestKey::ModifiedChar {
            ch: ' ',
            modifiers: key_modifiers,
        },
        Key::Named(NamedKey::ArrowUp) => {
            if let Some(bytes) = encode_modified_csi_final(b'A', key_modifiers) {
                return Some(bytes);
            }
            TestKey::ArrowUp
        }
        Key::Named(NamedKey::ArrowDown) => {
            if let Some(bytes) = encode_modified_csi_final(b'B', key_modifiers) {
                return Some(bytes);
            }
            TestKey::ArrowDown
        }
        Key::Named(NamedKey::ArrowRight) => {
            if let Some(bytes) = encode_modified_csi_final(b'C', key_modifiers) {
                return Some(bytes);
            }
            TestKey::ArrowRight
        }
        Key::Named(NamedKey::ArrowLeft) => {
            if let Some(bytes) = encode_modified_csi_final(b'D', key_modifiers) {
                return Some(bytes);
            }
            TestKey::ArrowLeft
        }
        Key::Named(NamedKey::Tab) if key_modifiers == KeyModifiers::SHIFT => {
            return Some(b"\x1b[Z".to_vec());
        }
        Key::Named(NamedKey::Tab) if key_modifiers.contains(KeyModifiers::SHIFT) => {
            return encode_modified_csi_final(b'Z', key_modifiers);
        }
        Key::Named(NamedKey::Tab) => TestKey::Tab,
        Key::Named(NamedKey::Home) => {
            return Some(
                encode_modified_csi_final(b'H', key_modifiers)
                    .unwrap_or_else(|| b"\x1b[H".to_vec()),
            );
        }
        Key::Named(NamedKey::End) => {
            return Some(
                encode_modified_csi_final(b'F', key_modifiers)
                    .unwrap_or_else(|| b"\x1b[F".to_vec()),
            );
        }
        Key::Named(NamedKey::Insert) => return Some(encode_tilde_key(2, key_modifiers)),
        Key::Named(NamedKey::Delete) => return Some(encode_tilde_key(3, key_modifiers)),
        Key::Named(NamedKey::PageUp) => return Some(encode_tilde_key(5, key_modifiers)),
        Key::Named(NamedKey::PageDown) => return Some(encode_tilde_key(6, key_modifiers)),
        Key::Named(NamedKey::F1) => return Some(encode_function_key(b'P', 1, key_modifiers)),
        Key::Named(NamedKey::F2) => return Some(encode_function_key(b'Q', 1, key_modifiers)),
        Key::Named(NamedKey::F3) => return Some(encode_function_key(b'R', 1, key_modifiers)),
        Key::Named(NamedKey::F4) => return Some(encode_function_key(b'S', 1, key_modifiers)),
        Key::Named(NamedKey::F5) => return Some(encode_tilde_key(15, key_modifiers)),
        Key::Named(NamedKey::F6) => return Some(encode_tilde_key(17, key_modifiers)),
        Key::Named(NamedKey::F7) => return Some(encode_tilde_key(18, key_modifiers)),
        Key::Named(NamedKey::F8) => return Some(encode_tilde_key(19, key_modifiers)),
        Key::Named(NamedKey::F9) => return Some(encode_tilde_key(20, key_modifiers)),
        Key::Named(NamedKey::F10) => return Some(encode_tilde_key(21, key_modifiers)),
        Key::Named(NamedKey::F11) => return Some(encode_tilde_key(23, key_modifiers)),
        Key::Named(NamedKey::F12) => return Some(encode_tilde_key(24, key_modifiers)),
        Key::Named(NamedKey::F13) => return Some(encode_shifted_function_key(1, key_modifiers)),
        Key::Named(NamedKey::F14) => return Some(encode_shifted_function_key(2, key_modifiers)),
        Key::Named(NamedKey::F15) => return Some(encode_shifted_function_key(3, key_modifiers)),
        Key::Named(NamedKey::F16) => return Some(encode_shifted_function_key(4, key_modifiers)),
        Key::Named(NamedKey::F17) => return Some(encode_shifted_function_key(5, key_modifiers)),
        Key::Named(NamedKey::F18) => return Some(encode_shifted_function_key(6, key_modifiers)),
        Key::Named(NamedKey::F19) => return Some(encode_shifted_function_key(7, key_modifiers)),
        Key::Named(NamedKey::F20) => return Some(encode_shifted_function_key(8, key_modifiers)),
        Key::Named(NamedKey::F21) => return Some(encode_shifted_function_key(9, key_modifiers)),
        Key::Named(NamedKey::F22) => return Some(encode_shifted_function_key(10, key_modifiers)),
        Key::Named(NamedKey::F23) => return Some(encode_shifted_function_key(11, key_modifiers)),
        Key::Named(NamedKey::F24) => return Some(encode_shifted_function_key(12, key_modifiers)),
        Key::Named(_) | Key::Unidentified(_) | Key::Dead(_) => return None,
    };
    Some(encode_keys_with_application_cursor_mode(
        &[key],
        application_cursor_keys,
    ))
}

fn encode_winit_keypad_key(
    physical_key: PhysicalKey,
    modifiers: KeyModifiers,
    application_keypad: bool,
) -> Option<Vec<u8>> {
    let PhysicalKey::Code(code) = physical_key else {
        return None;
    };

    if application_keypad {
        let final_byte = match code {
            KeyCode::Numpad0 => b'p',
            KeyCode::Numpad1 => b'q',
            KeyCode::Numpad2 => b'r',
            KeyCode::Numpad3 => b's',
            KeyCode::Numpad4 => b't',
            KeyCode::Numpad5 => b'u',
            KeyCode::Numpad6 => b'v',
            KeyCode::Numpad7 => b'w',
            KeyCode::Numpad8 => b'x',
            KeyCode::Numpad9 => b'y',
            KeyCode::NumpadDecimal => b'n',
            KeyCode::NumpadDivide => b'o',
            KeyCode::NumpadMultiply => b'j',
            KeyCode::NumpadSubtract => b'm',
            KeyCode::NumpadAdd => b'k',
            KeyCode::NumpadEnter => b'M',
            KeyCode::NumpadEqual => b'X',
            _ => return None,
        };
        return Some(encode_alt_prefixed_key(
            &[0x1b, b'O', final_byte],
            modifiers,
        ));
    }

    let ch = match code {
        KeyCode::Numpad0 => '0',
        KeyCode::Numpad1 => '1',
        KeyCode::Numpad2 => '2',
        KeyCode::Numpad3 => '3',
        KeyCode::Numpad4 => '4',
        KeyCode::Numpad5 => '5',
        KeyCode::Numpad6 => '6',
        KeyCode::Numpad7 => '7',
        KeyCode::Numpad8 => '8',
        KeyCode::Numpad9 => '9',
        KeyCode::NumpadDecimal => '.',
        KeyCode::NumpadDivide => '/',
        KeyCode::NumpadMultiply => '*',
        KeyCode::NumpadSubtract => '-',
        KeyCode::NumpadAdd => '+',
        KeyCode::NumpadEqual => '=',
        KeyCode::NumpadEnter => return Some(encode_alt_prefixed_key(b"\r", modifiers)),
        _ => return None,
    };

    let mut bytes = Vec::new();
    encode_modified_char(&mut bytes, ch, modifiers);
    Some(bytes)
}

/// Convert `winit` modifier state to terminal modifier flags.
pub(crate) fn key_modifiers_from_winit(modifiers: ModifiersState) -> KeyModifiers {
    let mut encoded = KeyModifiers::empty();
    if modifiers.shift_key() {
        encoded |= KeyModifiers::SHIFT;
    }
    if modifiers.control_key() {
        encoded |= KeyModifiers::CTRL;
    }
    if modifiers.alt_key() {
        encoded |= KeyModifiers::ALT;
    }
    if modifiers.super_key() {
        encoded |= KeyModifiers::SUPER;
    }
    encoded
}

fn encode_modified_char(bytes: &mut Vec<u8>, ch: char, modifiers: KeyModifiers) {
    if modifiers.contains(KeyModifiers::ALT) {
        bytes.push(0x1b);
    }
    if modifiers.contains(KeyModifiers::CTRL)
        && let Some(control) = control_byte(ch)
    {
        bytes.push(control);
        return;
    }
    push_char(bytes, ch);
}

fn encode_function_key(final_byte: u8, tilde_prefix: u8, modifiers: KeyModifiers) -> Vec<u8> {
    if let Some(bytes) = encode_modified_csi_final(final_byte, modifiers) {
        return bytes;
    }
    match final_byte {
        b'P' => b"\x1bOP".to_vec(),
        b'Q' => b"\x1bOQ".to_vec(),
        b'R' => b"\x1bOR".to_vec(),
        b'S' => b"\x1bOS".to_vec(),
        _ => encode_tilde_key(tilde_prefix, modifiers),
    }
}

fn encode_alt_prefixed_key(base: &[u8], modifiers: KeyModifiers) -> Vec<u8> {
    let mut bytes =
        Vec::with_capacity(base.len() + usize::from(modifiers.contains(KeyModifiers::ALT)));
    if modifiers.contains(KeyModifiers::ALT) {
        bytes.push(0x1b);
    }
    bytes.extend_from_slice(base);
    bytes
}

fn encode_shifted_function_key(function_number: u8, modifiers: KeyModifiers) -> Vec<u8> {
    let modifiers = modifiers | KeyModifiers::SHIFT;
    match function_number {
        1 => encode_function_key(b'P', 1, modifiers),
        2 => encode_function_key(b'Q', 1, modifiers),
        3 => encode_function_key(b'R', 1, modifiers),
        4 => encode_function_key(b'S', 1, modifiers),
        5 => encode_tilde_key(15, modifiers),
        6 => encode_tilde_key(17, modifiers),
        7 => encode_tilde_key(18, modifiers),
        8 => encode_tilde_key(19, modifiers),
        9 => encode_tilde_key(20, modifiers),
        10 => encode_tilde_key(21, modifiers),
        11 => encode_tilde_key(23, modifiers),
        12 => encode_tilde_key(24, modifiers),
        _ => unreachable!("shifted function key number is matched by caller"),
    }
}

fn encode_modified_csi_final(final_byte: u8, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    let modifier = xterm_modifier_parameter(modifiers)?;
    Some(format!("\x1b[1;{}{}", modifier, char::from(final_byte)).into_bytes())
}

fn encode_tilde_key(prefix: u8, modifiers: KeyModifiers) -> Vec<u8> {
    if let Some(modifier) = xterm_modifier_parameter(modifiers) {
        format!("\x1b[{prefix};{modifier}~").into_bytes()
    } else {
        format!("\x1b[{prefix}~").into_bytes()
    }
}

fn xterm_modifier_parameter(modifiers: KeyModifiers) -> Option<u8> {
    let mut parameter = 1;
    if modifiers.contains(KeyModifiers::SHIFT) {
        parameter += 1;
    }
    if modifiers.contains(KeyModifiers::ALT) {
        parameter += 2;
    }
    if modifiers.contains(KeyModifiers::CTRL) {
        parameter += 4;
    }
    (parameter != 1).then_some(parameter)
}

fn push_char(bytes: &mut Vec<u8>, ch: char) {
    let mut buffer = [0; 4];
    bytes.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
}

fn control_byte(ch: char) -> Option<u8> {
    let lower = ch.to_ascii_lowercase();
    if lower.is_ascii_lowercase() {
        Some((lower as u8) - b'a' + 1)
    } else {
        match ch {
            ' ' | '2' | '@' => Some(0x00),
            '[' | '3' => Some(0x1b),
            '\\' | '4' => Some(0x1c),
            ']' | '5' => Some(0x1d),
            '^' | '6' => Some(0x1e),
            '_' | '7' | '/' => Some(0x1f),
            '8' | '?' => Some(0x7f),
            _ => None,
        }
    }
}
