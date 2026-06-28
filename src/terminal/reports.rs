mod decrqss;

use super::Terminal;

impl Terminal {
    fn mode_state(&self, mode: u16) -> Option<bool> {
        match mode {
            4 => Some(self.insert_mode),
            20 => Some(self.linefeed_newline_mode),
            _ => None,
        }
    }

    pub(super) fn report_mode_state(&mut self, private: bool, mode: u16) {
        let state = if private {
            self.private_mode_state(mode)
        } else {
            self.mode_state(mode)
        };
        let value = match state {
            Some(true) => 1,
            Some(false) => 2,
            None => 0,
        };

        if private {
            self.pending_response_bytes
                .extend_from_slice(format!("\x1b[?{};{}$y", mode, value).as_bytes());
        } else {
            self.pending_response_bytes
                .extend_from_slice(format!("\x1b[{};{}$y", mode, value).as_bytes());
        }
    }

    pub(super) fn report_device_status(&mut self, mode: u16) {
        match mode {
            5 => self.pending_response_bytes.extend_from_slice(b"\x1b[0n"),
            6 => self.pending_response_bytes.extend_from_slice(
                format!("\x1b[{};{}R", self.cursor.row + 1, self.cursor.col + 1).as_bytes(),
            ),
            _ => {}
        }
    }

    pub(super) fn report_private_device_status(&mut self, mode: u16) {
        match mode {
            6 => self.pending_response_bytes.extend_from_slice(
                format!("\x1b[?{};{}R", self.cursor.row + 1, self.cursor.col + 1).as_bytes(),
            ),
            15 => self.pending_response_bytes.extend_from_slice(b"\x1b[?11n"),
            25 => self.pending_response_bytes.extend_from_slice(b"\x1b[?20n"),
            26 => self
                .pending_response_bytes
                .extend_from_slice(b"\x1b[?27;1;0;0n"),
            53 => self.pending_response_bytes.extend_from_slice(b"\x1b[?50n"),
            _ => {}
        }
    }

    pub(super) fn report_terminal_parameters(&mut self, mode: u16) {
        match mode {
            0 | 1 => self
                .pending_response_bytes
                .extend_from_slice(format!("\x1b[{};1;1;128;128;1;0x", mode + 2).as_bytes()),
            _ => {}
        }
    }

    pub(super) fn report_window_manipulation(&mut self, mode: u16) {
        match mode {
            11 => self.pending_response_bytes.extend_from_slice(b"\x1b[1t"),
            13 => self
                .pending_response_bytes
                .extend_from_slice(b"\x1b[3;0;0t"),
            14 => self.pending_response_bytes.extend_from_slice(
                format!(
                    "\x1b[4;{};{}t",
                    self.config.pixel_height, self.config.pixel_width
                )
                .as_bytes(),
            ),
            15 => self.pending_response_bytes.extend_from_slice(
                format!(
                    "\x1b[5;{};{}t",
                    self.config.pixel_height, self.config.pixel_width
                )
                .as_bytes(),
            ),
            18 => self.pending_response_bytes.extend_from_slice(
                format!("\x1b[8;{};{}t", self.config.rows, self.config.cols).as_bytes(),
            ),
            19 => self.pending_response_bytes.extend_from_slice(
                format!("\x1b[9;{};{}t", self.config.rows, self.config.cols).as_bytes(),
            ),
            20 => {
                self.pending_response_bytes.extend_from_slice(b"\x1b]L");
                if let Some(icon_label) = self.icon_label.as_ref().or(self.title.as_ref()) {
                    self.pending_response_bytes
                        .extend_from_slice(icon_label.as_bytes());
                }
                self.pending_response_bytes.extend_from_slice(b"\x1b\\");
            }
            21 => {
                self.pending_response_bytes.extend_from_slice(b"\x1b]l");
                if let Some(title) = &self.title {
                    self.pending_response_bytes
                        .extend_from_slice(title.as_bytes());
                }
                self.pending_response_bytes.extend_from_slice(b"\x1b\\");
            }
            _ => {}
        }
    }

    pub(super) fn report_primary_device_attributes(&mut self, mode: u16) {
        if mode == 0 {
            self.pending_response_bytes.extend_from_slice(b"\x1b[?1;2c");
        }
    }

    pub(super) fn report_secondary_device_attributes(&mut self, mode: u16) {
        if mode == 0 {
            self.pending_response_bytes
                .extend_from_slice(b"\x1b[>0;1;0c");
        }
    }

    #[cold]
    #[inline(never)]
    pub(super) fn report_decid(&mut self) {
        self.report_primary_device_attributes(0);
    }
}
