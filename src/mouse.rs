//! Mouse reporting state and event encoding.

/// Mouse reporting protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseProtocol {
    /// X10/default protocol is not emitted by this foundation slice.
    Default,
    /// SGR 1006 protocol.
    Sgr,
}

/// Mouse reporting state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseReportState {
    mode: MouseReportMode,
    protocol: MouseProtocol,
}

impl Default for MouseReportState {
    fn default() -> Self {
        Self {
            mode: MouseReportMode::Disabled,
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
        self.mode = if enabled {
            MouseReportMode::Button
        } else {
            MouseReportMode::Disabled
        };
    }

    /// Enable or disable button-motion reporting.
    pub fn set_button_motion_reporting(&mut self, enabled: bool) {
        self.mode = if enabled {
            MouseReportMode::ButtonMotion
        } else {
            MouseReportMode::Button
        };
    }

    /// Enable or disable any-motion reporting.
    pub fn set_any_motion_reporting(&mut self, enabled: bool) {
        self.mode = if enabled {
            MouseReportMode::AnyMotion
        } else {
            MouseReportMode::Button
        };
    }

    /// Enable or disable SGR mouse encoding.
    pub fn set_sgr_protocol(&mut self, enabled: bool) {
        self.protocol = if enabled {
            MouseProtocol::Sgr
        } else {
            MouseProtocol::Default
        };
    }

    /// Whether button-event reporting is active.
    pub fn button_reporting_enabled(self) -> bool {
        self.mode != MouseReportMode::Disabled
    }

    /// Whether button-motion reporting is active.
    pub fn button_motion_reporting_enabled(self) -> bool {
        self.mode == MouseReportMode::ButtonMotion
    }

    /// Whether any-motion reporting is active.
    pub fn any_motion_reporting_enabled(self) -> bool {
        self.mode == MouseReportMode::AnyMotion
    }

    /// Whether SGR mouse encoding is active.
    pub fn sgr_protocol_enabled(self) -> bool {
        self.protocol == MouseProtocol::Sgr
    }

    /// Encode a mouse event according to active reporting modes.
    pub fn encode(self, event: MouseEvent) -> Option<Vec<u8>> {
        if self.protocol != MouseProtocol::Sgr || !self.mode.reports(event.kind) {
            return None;
        }
        let suffix = match event.kind {
            MouseEventKind::Press | MouseEventKind::Drag | MouseEventKind::Motion => 'M',
            MouseEventKind::Release => 'm',
        };
        let code = event.button.code() + event.kind.motion_code_offset();
        Some(
            format!(
                "\x1b[<{};{};{}{}",
                code,
                event.col + 1,
                event.row + 1,
                suffix
            )
            .into_bytes(),
        )
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

impl MouseEventKind {
    fn motion_code_offset(self) -> u16 {
        match self {
            Self::Drag | Self::Motion => 32,
            Self::Press | Self::Release => 0,
        }
    }
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

/// Grid-relative mouse event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    /// Event kind.
    pub kind: MouseEventKind,
    /// Button identity.
    pub button: MouseButton,
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
            col,
            row,
        }
    }
}
