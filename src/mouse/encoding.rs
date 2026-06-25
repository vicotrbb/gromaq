use crate::input::KeyModifiers;

use super::{MouseButton, MouseEvent, MouseEventKind, MouseProtocol};

pub(super) fn encode_mouse_event(protocol: MouseProtocol, event: MouseEvent) -> Option<Vec<u8>> {
    let modifier_code = mouse_modifier_code(event.modifiers);
    let code = event.button.code() + event.kind.motion_code_offset() + modifier_code;
    match protocol {
        MouseProtocol::Default => {
            let code = if event.kind == MouseEventKind::Release {
                MouseButton::None.code() + modifier_code
            } else {
                code
            };
            encode_default_mouse_event(code, event)
        }
        MouseProtocol::Sgr => {
            let col = event.col.checked_add(1)?;
            let row = event.row.checked_add(1)?;
            let suffix = match event.kind {
                MouseEventKind::Press | MouseEventKind::Drag | MouseEventKind::Motion => 'M',
                MouseEventKind::Release => 'm',
            };
            Some(format!("\x1b[<{};{};{}{}", code, col, row, suffix).into_bytes())
        }
    }
}

fn encode_default_mouse_event(code: u16, event: MouseEvent) -> Option<Vec<u8>> {
    Some(vec![
        0x1b,
        b'[',
        b'M',
        default_mouse_byte(code)?,
        default_mouse_byte(event.col.checked_add(1)?)?,
        default_mouse_byte(event.row.checked_add(1)?)?,
    ])
}

fn default_mouse_byte(value: u16) -> Option<u8> {
    u8::try_from(value.checked_add(32)?).ok()
}

fn mouse_modifier_code(modifiers: KeyModifiers) -> u16 {
    let mut code = 0;
    if modifiers.contains(KeyModifiers::SHIFT) {
        code += 4;
    }
    if modifiers.contains(KeyModifiers::ALT) {
        code += 8;
    }
    if modifiers.contains(KeyModifiers::CTRL) {
        code += 16;
    }
    code
}

impl MouseEventKind {
    fn motion_code_offset(self) -> u16 {
        match self {
            Self::Drag | Self::Motion => 32,
            Self::Press | Self::Release => 0,
        }
    }
}

impl MouseButton {
    fn code(self) -> u16 {
        match self {
            Self::None => 3,
            Self::Left => 0,
            Self::Middle => 1,
            Self::Right => 2,
            Self::WheelUp => 64,
            Self::WheelDown => 65,
        }
    }
}
