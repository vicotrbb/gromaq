//! Native input, mouse, and resize mapping helpers for the app loop.

use winit::keyboard::{Key, ModifiersState, NamedKey};

mod mouse;
mod resize;
mod shortcuts;

pub use mouse::{
    NativeMouseButtonTracker, NativeMouseGridMapper, NativeRenderedGridMetrics,
    NativeWindowMouseInput, NativeWindowMouseInputResult,
};
pub(super) use mouse::{native_mouse_button, wheel_mouse_button};
pub(super) use resize::clamp_u32_to_u16;
pub use resize::{NativePtyResize, NativeResizeGridMapper};
pub use shortcuts::{
    NativeTextZoomAction, is_native_copy_shortcut, is_native_paste_shortcut,
    native_text_zoom_action, native_wheel_text_zoom_action,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScrollbackKeyDirection {
    Up,
    Down,
}

pub(super) fn native_scrollback_key_direction(
    key: &Key,
    modifiers: ModifiersState,
) -> Option<ScrollbackKeyDirection> {
    if !modifiers.shift_key()
        || modifiers.control_key()
        || modifiers.alt_key()
        || modifiers.super_key()
    {
        return None;
    }

    match key {
        Key::Named(NamedKey::PageUp) => Some(ScrollbackKeyDirection::Up),
        Key::Named(NamedKey::PageDown) => Some(ScrollbackKeyDirection::Down),
        _ => None,
    }
}
