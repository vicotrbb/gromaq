//! Native input, mouse, and resize mapping helpers for the app loop.

use winit::event::{MouseButton as WinitMouseButton, MouseScrollDelta};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::input::key_modifiers_from_winit;
use crate::mouse::{MouseButton, MouseEvent, MouseEventKind};

mod resize;
mod shortcuts;

pub(super) use resize::clamp_u32_to_u16;
pub use resize::{NativePtyResize, NativeResizeGridMapper};
pub use shortcuts::{
    NativeTextZoomAction, is_native_copy_shortcut, is_native_paste_shortcut,
    native_text_zoom_action, native_wheel_text_zoom_action,
};

/// Maps native window pixel positions to terminal grid-relative mouse events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeMouseGridMapper {
    window_width_px: u32,
    window_height_px: u32,
    cell_width_px: u16,
    line_height_px: u16,
    surface_padding_px: u16,
    cols: u16,
    rows: u16,
}

/// Native window mouse input before terminal grid mapping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeWindowMouseInput {
    /// Window-relative x coordinate in physical pixels.
    pub x: f64,
    /// Window-relative y coordinate in physical pixels.
    pub y: f64,
    /// Current window width in physical pixels.
    pub window_width_px: u32,
    /// Current window height in physical pixels.
    pub window_height_px: u32,
    /// Rendered terminal cell width in physical pixels.
    pub cell_width_px: u16,
    /// Rendered terminal row height in physical pixels.
    pub line_height_px: u16,
    /// Empty space around rendered terminal cells in physical pixels.
    pub surface_padding_px: u16,
    /// Mouse event kind.
    pub kind: MouseEventKind,
    /// Mouse button identity.
    pub button: MouseButton,
    /// Active keyboard modifiers.
    pub modifiers: ModifiersState,
}

/// Tracks currently pressed native mouse buttons for cursor-move reporting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeMouseButtonTracker {
    left: bool,
    middle: bool,
    right: bool,
}

impl NativeMouseButtonTracker {
    /// Record a native button press or release.
    pub fn set_pressed(&mut self, button: MouseButton, pressed: bool) {
        match button {
            MouseButton::Left => self.left = pressed,
            MouseButton::Middle => self.middle = pressed,
            MouseButton::Right => self.right = pressed,
            MouseButton::None | MouseButton::WheelUp | MouseButton::WheelDown => {}
        }
    }

    /// Mouse event kind and button identity to use for a cursor-move event.
    pub fn cursor_move_event(self) -> (MouseEventKind, MouseButton) {
        if self.left {
            (MouseEventKind::Drag, MouseButton::Left)
        } else if self.middle {
            (MouseEventKind::Drag, MouseButton::Middle)
        } else if self.right {
            (MouseEventKind::Drag, MouseButton::Right)
        } else {
            (MouseEventKind::Motion, MouseButton::None)
        }
    }
}

impl NativeMouseGridMapper {
    /// Create a mapper for a non-empty window and terminal grid.
    pub fn new(
        window_width_px: u32,
        window_height_px: u32,
        cell_width_px: u16,
        line_height_px: u16,
        surface_padding_px: u16,
        cols: u16,
        rows: u16,
    ) -> Option<Self> {
        if window_width_px == 0
            || window_height_px == 0
            || cell_width_px == 0
            || line_height_px == 0
            || cols == 0
            || rows == 0
        {
            return None;
        }
        Some(Self {
            window_width_px,
            window_height_px,
            cell_width_px,
            line_height_px,
            surface_padding_px,
            cols,
            rows,
        })
    }

    /// Convert a window pixel position to a grid-relative terminal mouse event.
    pub fn mouse_event_at(
        self,
        x: f64,
        y: f64,
        kind: MouseEventKind,
        button: MouseButton,
    ) -> Option<MouseEvent> {
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.0
            || y < 0.0
            || x >= f64::from(self.window_width_px)
            || y >= f64::from(self.window_height_px)
        {
            return None;
        }
        let grid_x = x - f64::from(self.surface_padding_px);
        let grid_y = y - f64::from(self.surface_padding_px);
        if grid_x < 0.0 || grid_y < 0.0 {
            return None;
        }
        let grid_width_px = f64::from(self.cell_width_px) * f64::from(self.cols);
        let grid_height_px = f64::from(self.line_height_px) * f64::from(self.rows);
        if grid_x >= grid_width_px || grid_y >= grid_height_px {
            return None;
        }
        let col = (grid_x / f64::from(self.cell_width_px)) as u16;
        let row = (grid_y / f64::from(self.line_height_px)) as u16;
        Some(MouseEvent::new(kind, button, col, row))
    }

    /// Convert a window pixel position to a grid-relative mouse event with modifiers.
    pub fn mouse_event_at_with_modifiers(
        self,
        x: f64,
        y: f64,
        kind: MouseEventKind,
        button: MouseButton,
        modifiers: ModifiersState,
    ) -> Option<MouseEvent> {
        self.mouse_event_at(x, y, kind, button)
            .map(|event| event.with_modifiers(key_modifiers_from_winit(modifiers)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScrollbackKeyDirection {
    Up,
    Down,
}

pub(super) fn native_mouse_button(button: WinitMouseButton) -> Option<MouseButton> {
    match button {
        WinitMouseButton::Left => Some(MouseButton::Left),
        WinitMouseButton::Middle => Some(MouseButton::Middle),
        WinitMouseButton::Right => Some(MouseButton::Right),
        WinitMouseButton::Back | WinitMouseButton::Forward | WinitMouseButton::Other(_) => None,
    }
}

pub(super) fn wheel_mouse_button(delta: &MouseScrollDelta) -> Option<MouseButton> {
    let y = match delta {
        MouseScrollDelta::LineDelta(_, y) => *y,
        MouseScrollDelta::PixelDelta(position) => position.y as f32,
    };
    if y > 0.0 {
        Some(MouseButton::WheelUp)
    } else if y < 0.0 {
        Some(MouseButton::WheelDown)
    } else {
        None
    }
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
