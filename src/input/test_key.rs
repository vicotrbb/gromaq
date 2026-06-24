use super::{KeyModifiers, encode_modified_char, push_char};

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

pub(super) fn encode_keys_with_application_cursor_mode(
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
