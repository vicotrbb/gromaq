//! Named-key mapping for `winit` keyboard input.

use winit::keyboard::NamedKey;

use crate::input::{
    KeyModifiers, TestKey, encode_alt_prefixed_key, encode_function_key, encode_modified_csi_final,
    encode_shifted_function_key, encode_tilde_key,
};

pub(super) enum EncodedNamedKey {
    Bytes(Vec<u8>),
    TestKey(TestKey),
    Ignored,
}

pub(super) fn encode_named_key(named_key: &NamedKey, modifiers: KeyModifiers) -> EncodedNamedKey {
    match named_key {
        NamedKey::Enter => EncodedNamedKey::Bytes(encode_alt_prefixed_key(b"\r", modifiers)),
        NamedKey::Backspace => EncodedNamedKey::Bytes(encode_alt_prefixed_key(b"\x7f", modifiers)),
        NamedKey::Escape => EncodedNamedKey::Bytes(encode_alt_prefixed_key(b"\x1b", modifiers)),
        NamedKey::Space => EncodedNamedKey::TestKey(TestKey::ModifiedChar { ch: ' ', modifiers }),
        NamedKey::ArrowUp => encode_arrow_key(b'A', modifiers, TestKey::ArrowUp),
        NamedKey::ArrowDown => encode_arrow_key(b'B', modifiers, TestKey::ArrowDown),
        NamedKey::ArrowRight => encode_arrow_key(b'C', modifiers, TestKey::ArrowRight),
        NamedKey::ArrowLeft => encode_arrow_key(b'D', modifiers, TestKey::ArrowLeft),
        NamedKey::Tab if modifiers == KeyModifiers::SHIFT => {
            EncodedNamedKey::Bytes(b"\x1b[Z".to_vec())
        }
        NamedKey::Tab if modifiers.contains(KeyModifiers::SHIFT) => {
            encode_optional_bytes(encode_modified_csi_final(b'Z', modifiers))
        }
        NamedKey::Tab => EncodedNamedKey::TestKey(TestKey::Tab),
        NamedKey::Home => EncodedNamedKey::Bytes(
            encode_modified_csi_final(b'H', modifiers).unwrap_or_else(|| b"\x1b[H".to_vec()),
        ),
        NamedKey::End => EncodedNamedKey::Bytes(
            encode_modified_csi_final(b'F', modifiers).unwrap_or_else(|| b"\x1b[F".to_vec()),
        ),
        NamedKey::Insert => EncodedNamedKey::Bytes(encode_tilde_key(2, modifiers)),
        NamedKey::Delete => EncodedNamedKey::Bytes(encode_tilde_key(3, modifiers)),
        NamedKey::PageUp => EncodedNamedKey::Bytes(encode_tilde_key(5, modifiers)),
        NamedKey::PageDown => EncodedNamedKey::Bytes(encode_tilde_key(6, modifiers)),
        NamedKey::F1 => EncodedNamedKey::Bytes(encode_function_key(b'P', 1, modifiers)),
        NamedKey::F2 => EncodedNamedKey::Bytes(encode_function_key(b'Q', 1, modifiers)),
        NamedKey::F3 => EncodedNamedKey::Bytes(encode_function_key(b'R', 1, modifiers)),
        NamedKey::F4 => EncodedNamedKey::Bytes(encode_function_key(b'S', 1, modifiers)),
        NamedKey::F5 => EncodedNamedKey::Bytes(encode_tilde_key(15, modifiers)),
        NamedKey::F6 => EncodedNamedKey::Bytes(encode_tilde_key(17, modifiers)),
        NamedKey::F7 => EncodedNamedKey::Bytes(encode_tilde_key(18, modifiers)),
        NamedKey::F8 => EncodedNamedKey::Bytes(encode_tilde_key(19, modifiers)),
        NamedKey::F9 => EncodedNamedKey::Bytes(encode_tilde_key(20, modifiers)),
        NamedKey::F10 => EncodedNamedKey::Bytes(encode_tilde_key(21, modifiers)),
        NamedKey::F11 => EncodedNamedKey::Bytes(encode_tilde_key(23, modifiers)),
        NamedKey::F12 => EncodedNamedKey::Bytes(encode_tilde_key(24, modifiers)),
        NamedKey::F13 => encode_optional_bytes(encode_shifted_function_key(1, modifiers)),
        NamedKey::F14 => encode_optional_bytes(encode_shifted_function_key(2, modifiers)),
        NamedKey::F15 => encode_optional_bytes(encode_shifted_function_key(3, modifiers)),
        NamedKey::F16 => encode_optional_bytes(encode_shifted_function_key(4, modifiers)),
        NamedKey::F17 => encode_optional_bytes(encode_shifted_function_key(5, modifiers)),
        NamedKey::F18 => encode_optional_bytes(encode_shifted_function_key(6, modifiers)),
        NamedKey::F19 => encode_optional_bytes(encode_shifted_function_key(7, modifiers)),
        NamedKey::F20 => encode_optional_bytes(encode_shifted_function_key(8, modifiers)),
        NamedKey::F21 => encode_optional_bytes(encode_shifted_function_key(9, modifiers)),
        NamedKey::F22 => encode_optional_bytes(encode_shifted_function_key(10, modifiers)),
        NamedKey::F23 => encode_optional_bytes(encode_shifted_function_key(11, modifiers)),
        NamedKey::F24 => encode_optional_bytes(encode_shifted_function_key(12, modifiers)),
        _ => EncodedNamedKey::Ignored,
    }
}

fn encode_arrow_key(final_byte: u8, modifiers: KeyModifiers, fallback: TestKey) -> EncodedNamedKey {
    match encode_modified_csi_final(final_byte, modifiers) {
        Some(bytes) => EncodedNamedKey::Bytes(bytes),
        None => EncodedNamedKey::TestKey(fallback),
    }
}

fn encode_optional_bytes(bytes: Option<Vec<u8>>) -> EncodedNamedKey {
    match bytes {
        Some(bytes) => EncodedNamedKey::Bytes(bytes),
        None => EncodedNamedKey::Ignored,
    }
}
