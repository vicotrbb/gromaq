//! Native input, mouse, and resize mapping helpers for the app loop.

use winit::event::{MouseButton as WinitMouseButton, MouseScrollDelta};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::input::key_modifiers_from_winit;
use crate::mouse::{MouseButton, MouseEvent, MouseEventKind};

/// Maps native window pixel positions to terminal grid-relative mouse events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeMouseGridMapper {
    window_width_px: u32,
    window_height_px: u32,
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

/// Terminal and PTY size requested by a native resize event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativePtyResize {
    /// Terminal columns.
    pub cols: u16,
    /// Terminal rows.
    pub rows: u16,
    /// Pixel width of the PTY viewport.
    pub pixel_width: u16,
    /// Pixel height of the PTY viewport.
    pub pixel_height: u16,
}

/// Maps native window pixel sizes to terminal row/column counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeResizeGridMapper {
    cell_width_px: u16,
    line_height_px: u16,
    surface_padding_px: u16,
}

impl NativeResizeGridMapper {
    /// Create a mapper from non-empty rendered cell metrics.
    pub fn new(cell_width_px: u16, line_height_px: u16, surface_padding_px: u16) -> Option<Self> {
        if cell_width_px == 0 || line_height_px == 0 {
            return None;
        }
        Some(Self {
            cell_width_px,
            line_height_px,
            surface_padding_px,
        })
    }

    /// Convert a native window size into a terminal and PTY resize request.
    pub fn resize_for_window(self, width_px: u32, height_px: u32) -> Option<NativePtyResize> {
        if width_px == 0 || height_px == 0 {
            return None;
        }
        let horizontal_padding = u32::from(self.surface_padding_px).saturating_mul(2);
        let vertical_padding = u32::from(self.surface_padding_px).saturating_mul(2);
        let cols = fitted_cells(
            width_px.saturating_sub(horizontal_padding),
            self.cell_width_px,
        );
        let rows = fitted_cells(
            height_px.saturating_sub(vertical_padding),
            self.line_height_px,
        );
        Some(NativePtyResize {
            cols,
            rows,
            pixel_width: clamp_u32_to_u16(width_px),
            pixel_height: clamp_u32_to_u16(height_px),
        })
    }
}

fn fitted_cells(available_px: u32, cell_px: u16) -> u16 {
    let cells = available_px / u32::from(cell_px);
    clamp_u32_to_u16(cells.max(1))
}

impl NativeMouseGridMapper {
    /// Create a mapper for a non-empty window and terminal grid.
    pub fn new(window_width_px: u32, window_height_px: u32, cols: u16, rows: u16) -> Option<Self> {
        if window_width_px == 0 || window_height_px == 0 || cols == 0 || rows == 0 {
            return None;
        }
        Some(Self {
            window_width_px,
            window_height_px,
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
        let col = ((x / f64::from(self.window_width_px)) * f64::from(self.cols)) as u16;
        let row = ((y / f64::from(self.window_height_px)) * f64::from(self.rows)) as u16;
        Some(MouseEvent::new(
            kind,
            button,
            col.min(self.cols - 1),
            row.min(self.rows - 1),
        ))
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

pub(super) fn clamp_u32_to_u16(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

pub(super) fn native_mouse_button(button: WinitMouseButton) -> Option<MouseButton> {
    match button {
        WinitMouseButton::Left => Some(MouseButton::Left),
        WinitMouseButton::Middle => Some(MouseButton::Middle),
        WinitMouseButton::Right => Some(MouseButton::Right),
        WinitMouseButton::Back | WinitMouseButton::Forward | WinitMouseButton::Other(_) => None,
    }
}

pub(super) fn wheel_mouse_button(delta: MouseScrollDelta) -> Option<MouseButton> {
    let y = match delta {
        MouseScrollDelta::LineDelta(_, y) => y,
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
