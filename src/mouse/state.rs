use super::{
    MouseEvent, MouseEventKind, MouseProtocol, MouseReportMode, MouseReportState, encoding,
};

impl MouseReportState {
    /// Enable or disable X10 button-press reporting.
    pub fn set_x10_reporting(&mut self, enabled: bool) {
        self.x10_reporting = enabled;
    }

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

    /// Whether DECSET 9 X10 button-press reporting is enabled.
    pub fn x10_reporting_enabled(self) -> bool {
        self.x10_reporting
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
        } else if self.x10_reporting {
            MouseReportMode::X10
        } else {
            MouseReportMode::Disabled
        }
    }
}

impl MouseReportMode {
    fn reports(self, kind: MouseEventKind) -> bool {
        match (self, kind) {
            (Self::Disabled, _) => false,
            (Self::X10, MouseEventKind::Press) => true,
            (
                Self::X10,
                MouseEventKind::Release | MouseEventKind::Drag | MouseEventKind::Motion,
            ) => false,
            (Self::Button, MouseEventKind::Press | MouseEventKind::Release) => true,
            (Self::Button, MouseEventKind::Drag | MouseEventKind::Motion) => false,
            (Self::ButtonMotion, MouseEventKind::Motion) => false,
            (Self::ButtonMotion, _) => true,
            (Self::AnyMotion, _) => true,
        }
    }
}
