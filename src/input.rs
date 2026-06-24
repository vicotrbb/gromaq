//! Keyboard input encoding.

use bitflags::bitflags;
pub use test_key::{TestKey, encode_keys};
pub use winit_keys::encode_winit_key;

mod keypad;
mod test_key;
mod winit_keys;
pub(crate) use winit_keys::{encode_winit_key_with_terminal_modes, key_modifiers_from_winit};

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

fn encode_shifted_function_key(function_number: u8, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    let modifiers = modifiers | KeyModifiers::SHIFT;
    let bytes = match function_number {
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
        _ => return None,
    };
    Some(bytes)
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
