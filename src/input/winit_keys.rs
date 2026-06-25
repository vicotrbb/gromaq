//! `winit` keyboard event encoding.

mod named;

use winit::keyboard::{Key, ModifiersState, PhysicalKey};

use super::keypad::encode_winit_keypad_key;
use super::test_key::encode_keys_with_application_cursor_mode;
use super::{KeyModifiers, TestKey, push_char};
use named::{EncodedNamedKey, encode_named_key};

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
                if key_modifiers.contains(KeyModifiers::ALT) {
                    bytes.push(0x1b);
                }
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
        Key::Named(named_key) => match encode_named_key(named_key, key_modifiers) {
            EncodedNamedKey::Bytes(bytes) => return Some(bytes),
            EncodedNamedKey::TestKey(key) => key,
            EncodedNamedKey::Ignored => return None,
        },
        Key::Unidentified(_) | Key::Dead(_) => return None,
    };
    Some(encode_keys_with_application_cursor_mode(
        &[key],
        application_cursor_keys,
    ))
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
