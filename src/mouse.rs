//! Mouse reporting state and event encoding.

use crate::input::KeyModifiers;

mod encoding;
mod state;

/// Mouse reporting protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseProtocol {
    /// Xterm default `CSI M Cb Cx Cy` protocol.
    Default,
    /// SGR 1006 protocol.
    Sgr,
}

/// Mouse reporting state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseReportState {
    button_reporting: bool,
    button_motion_reporting: bool,
    any_motion_reporting: bool,
    protocol: MouseProtocol,
}

impl Default for MouseReportState {
    fn default() -> Self {
        Self {
            button_reporting: false,
            button_motion_reporting: false,
            any_motion_reporting: false,
            protocol: MouseProtocol::Default,
        }
    }
}

/// Mouse reporting mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseReportMode {
    /// No mouse reports.
    Disabled,
    /// Button press/release reports.
    Button,
    /// Button press/release plus drag reports.
    ButtonMotion,
    /// Button press/release plus any motion reports.
    AnyMotion,
}

/// Mouse event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    /// Button press or wheel event.
    Press,
    /// Button release.
    Release,
    /// Button drag.
    Drag,
    /// Motion without a pressed button.
    Motion,
}

/// Mouse button identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// No button.
    None,
    /// Left button.
    Left,
    /// Middle button.
    Middle,
    /// Right button.
    Right,
    /// Wheel up event.
    WheelUp,
    /// Wheel down event.
    WheelDown,
}

/// Grid-relative mouse event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    /// Event kind.
    pub kind: MouseEventKind,
    /// Button identity.
    pub button: MouseButton,
    /// Active keyboard modifiers.
    pub modifiers: KeyModifiers,
    /// Zero-based column.
    pub col: u16,
    /// Zero-based row.
    pub row: u16,
}

impl MouseEvent {
    /// Create a mouse event.
    pub fn new(kind: MouseEventKind, button: MouseButton, col: u16, row: u16) -> Self {
        Self {
            kind,
            button,
            modifiers: KeyModifiers::empty(),
            col,
            row,
        }
    }

    /// Return this event with active keyboard modifiers attached.
    pub fn with_modifiers(mut self, modifiers: KeyModifiers) -> Self {
        self.modifiers = modifiers;
        self
    }
}
