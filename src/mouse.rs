//! Mouse reporting state and event encoding.

use crate::input::KeyModifiers;

mod encoding;

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

impl MouseReportState {
    /// Enable or disable button-event reporting.
    pub fn set_button_reporting(&mut self, enabled: bool) {
        self.button_reporting = enabled;
    }

    /// Enable or disable button-motion reporting.
    pub fn set_button_motion_reporting(&mut self, enabled: bool) {
        self.button_motion_reporting = enabled;
    }

    /// Enable or disable any-motion reporting.
    pub fn set_any_motion_reporting(&mut self, enabled: bool) {
        self.any_motion_reporting = enabled;
    }

    /// Enable or disable SGR mouse encoding.
    pub fn set_sgr_protocol(&mut self, enabled: bool) {
        self.protocol = if enabled {
            MouseProtocol::Sgr
        } else {
            MouseProtocol::Default
        };
    }

    /// Whether DECSET 1000 button-event reporting is enabled.
    pub fn button_reporting_enabled(self) -> bool {
        self.button_reporting
    }

    /// Whether DECSET 1002 button-motion reporting is enabled.
    pub fn button_motion_reporting_enabled(self) -> bool {
        self.button_motion_reporting
    }

    /// Whether DECSET 1003 any-motion reporting is enabled.
    pub fn any_motion_reporting_enabled(self) -> bool {
        self.any_motion_reporting
    }

    /// Whether SGR mouse encoding is active.
    pub fn sgr_protocol_enabled(self) -> bool {
        self.protocol == MouseProtocol::Sgr
    }

    /// Encode a mouse event according to active reporting modes.
    pub fn encode(self, event: MouseEvent) -> Option<Vec<u8>> {
        if !self.effective_mode().reports(event.kind) {
            return None;
        }
        encoding::encode_mouse_event(self.protocol, event)
    }

    fn effective_mode(self) -> MouseReportMode {
        if self.any_motion_reporting {
            MouseReportMode::AnyMotion
        } else if self.button_motion_reporting {
            MouseReportMode::ButtonMotion
        } else if self.button_reporting {
            MouseReportMode::Button
        } else {
            MouseReportMode::Disabled
        }
    }
}

impl MouseReportMode {
    fn reports(self, kind: MouseEventKind) -> bool {
        match (self, kind) {
            (Self::Disabled, _) => false,
            (Self::Button, MouseEventKind::Press | MouseEventKind::Release) => true,
            (Self::Button, MouseEventKind::Drag | MouseEventKind::Motion) => false,
            (Self::ButtonMotion, MouseEventKind::Motion) => false,
            (Self::ButtonMotion, _) => true,
            (Self::AnyMotion, _) => true,
        }
    }
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
