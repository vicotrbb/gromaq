use winit::event::{MouseButton as WinitMouseButton, MouseScrollDelta};
use winit::keyboard::ModifiersState;

use crate::input::key_modifiers_from_winit;
use crate::mouse::{MouseButton, MouseEvent, MouseEventKind};

/// Maps native window pixel positions to terminal grid-relative mouse events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeMouseGridMapper {
    window_width_px: u32,
    window_height_px: u32,
    metrics: NativeRenderedGridMetrics,
}

/// Rendered terminal grid metrics used by native input hit testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeRenderedGridMetrics {
    /// Rendered terminal cell width in physical pixels.
    pub cell_width_px: u16,
    /// Rendered terminal row height in physical pixels.
    pub line_height_px: u16,
    /// Empty space around rendered terminal cells in physical pixels.
    pub surface_padding_px: u16,
    /// Visual gap between adjacent rendered terminal cells in physical pixels.
    pub cell_spacing_px: u16,
    /// Visible terminal columns.
    pub cols: u16,
    /// Visible terminal rows.
    pub rows: u16,
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
    /// Visual gap between adjacent rendered terminal cells in physical pixels.
    pub cell_spacing_px: u16,
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
        metrics: NativeRenderedGridMetrics,
    ) -> Option<Self> {
        if window_width_px == 0
            || window_height_px == 0
            || metrics.cell_width_px == 0
            || metrics.line_height_px == 0
            || metrics.cols == 0
            || metrics.rows == 0
        {
            return None;
        }
        Some(Self {
            window_width_px,
            window_height_px,
            metrics,
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
        let grid_x = x - f64::from(self.metrics.surface_padding_px);
        let grid_y = y - f64::from(self.metrics.surface_padding_px);
        if grid_x < 0.0 || grid_y < 0.0 {
            return None;
        }
        let cell_pitch_x = f64::from(self.metrics.cell_width_px + self.metrics.cell_spacing_px);
        let cell_pitch_y = f64::from(self.metrics.line_height_px + self.metrics.cell_spacing_px);
        let grid_width_px = f64::from(self.metrics.cell_width_px) * f64::from(self.metrics.cols)
            + f64::from(self.metrics.cell_spacing_px)
                * f64::from(self.metrics.cols.saturating_sub(1));
        let grid_height_px = f64::from(self.metrics.line_height_px) * f64::from(self.metrics.rows)
            + f64::from(self.metrics.cell_spacing_px)
                * f64::from(self.metrics.rows.saturating_sub(1));
        if grid_x >= grid_width_px || grid_y >= grid_height_px {
            return None;
        }
        let col = (grid_x / cell_pitch_x) as u16;
        let row = (grid_y / cell_pitch_y) as u16;
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

pub(in crate::app) fn native_mouse_button(button: WinitMouseButton) -> Option<MouseButton> {
    match button {
        WinitMouseButton::Left => Some(MouseButton::Left),
        WinitMouseButton::Middle => Some(MouseButton::Middle),
        WinitMouseButton::Right => Some(MouseButton::Right),
        WinitMouseButton::Back | WinitMouseButton::Forward | WinitMouseButton::Other(_) => None,
    }
}

pub(in crate::app) fn wheel_mouse_button(delta: &MouseScrollDelta) -> Option<MouseButton> {
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
