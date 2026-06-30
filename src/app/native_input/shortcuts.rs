//! Native copy, paste, and text zoom shortcut policy.

use winit::event::MouseScrollDelta;
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

use crate::mouse::MouseButton;

use super::wheel_mouse_button;

/// Native text zoom action requested by app-owned keyboard shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTextZoomAction {
    /// Increase terminal text size.
    Increase,
    /// Decrease terminal text size.
    Decrease,
    /// Reset terminal text size to the default metrics.
    Reset,
}

/// Native tmux assist action requested by app-owned keyboard shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTmuxAssistAction {
    /// Toggle the native tmux manager panel.
    ToggleManager,
}

/// Browser-style terminal text zoom action requested by a modified mouse wheel event.
pub fn native_wheel_text_zoom_action(
    delta: &MouseScrollDelta,
    modifiers: ModifiersState,
) -> Option<NativeTextZoomAction> {
    let uses_zoom_modifier = modifiers.control_key() ^ modifiers.super_key();
    if !uses_zoom_modifier || modifiers.alt_key() || modifiers.shift_key() {
        return None;
    }
    match wheel_mouse_button(delta)? {
        MouseButton::WheelUp => Some(NativeTextZoomAction::Increase),
        MouseButton::WheelDown => Some(NativeTextZoomAction::Decrease),
        _ => None,
    }
}

/// Whether a native key event should copy the active terminal selection.
pub fn is_native_copy_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    matches!(key, Key::Named(NamedKey::Copy))
        || (matches!(key, Key::Named(NamedKey::Insert))
            && modifiers.control_key()
            && !modifiers.shift_key()
            && !modifiers.alt_key()
            && !modifiers.super_key())
        || (matches!(key, Key::Character(character) if character.eq_ignore_ascii_case("c"))
            && !modifiers.alt_key()
            && ((modifiers.super_key() && !modifiers.control_key())
                || (modifiers.control_key() && modifiers.shift_key() && !modifiers.super_key())))
}

/// Whether a native key event should paste from the host clipboard.
pub fn is_native_paste_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    matches!(key, Key::Named(NamedKey::Paste))
        || (matches!(key, Key::Named(NamedKey::Insert))
            && modifiers.shift_key()
            && !modifiers.control_key()
            && !modifiers.alt_key()
            && !modifiers.super_key())
        || (matches!(key, Key::Character(character) if character.eq_ignore_ascii_case("v"))
            && !modifiers.alt_key()
            && ((modifiers.control_key() && !modifiers.super_key())
                || (modifiers.super_key() && !modifiers.control_key())))
}

/// Native tmux assist shortcut for the terminal viewport.
pub fn native_tmux_assist_action(
    key: &Key,
    physical_key: Option<PhysicalKey>,
    modifiers: ModifiersState,
) -> Option<NativeTmuxAssistAction> {
    let command_modifier = modifiers.control_key() ^ modifiers.super_key();
    if !command_modifier || !modifiers.shift_key() || modifiers.alt_key() {
        return None;
    }
    if matches!(physical_key, Some(PhysicalKey::Code(KeyCode::KeyT))) {
        return Some(NativeTmuxAssistAction::ToggleManager);
    }
    match key {
        Key::Character(character) if character.eq_ignore_ascii_case("t") => {
            Some(NativeTmuxAssistAction::ToggleManager)
        }
        _ => None,
    }
}

/// Browser-style native text zoom shortcut for the terminal viewport.
pub fn native_text_zoom_action(
    key: &Key,
    modifiers: ModifiersState,
) -> Option<NativeTextZoomAction> {
    match key {
        Key::Named(NamedKey::ZoomIn) if modifiers.is_empty() => {
            return Some(NativeTextZoomAction::Increase);
        }
        Key::Named(NamedKey::ZoomOut) if modifiers.is_empty() => {
            return Some(NativeTextZoomAction::Decrease);
        }
        _ => {}
    }

    let command_modifier = modifiers.control_key() ^ modifiers.super_key();
    if !command_modifier || modifiers.alt_key() {
        return None;
    }
    let Key::Character(character) = key else {
        return None;
    };
    match character.as_ref() {
        "+" | "=" => Some(NativeTextZoomAction::Increase),
        "-" | "_" => Some(NativeTextZoomAction::Decrease),
        "0" => Some(NativeTextZoomAction::Reset),
        _ => None,
    }
}
