use crate::cell::{Color, UnderlineStyle};

use super::params::{push_sgr_color_parameters, push_sgr_extended_color_parameter};
use super::{CursorShape, Terminal};

impl Terminal {
    fn cursor_shape_parameter(&self) -> u16 {
        match (self.cursor.shape, self.cursor.blinking) {
            (CursorShape::Block, true) => 1,
            (CursorShape::Block, false) => 2,
            (CursorShape::Underline, true) => 3,
            (CursorShape::Underline, false) => 4,
            (CursorShape::Bar, true) => 5,
            (CursorShape::Bar, false) => 6,
        }
    }

    fn active_sgr_parameters(&self) -> Vec<String> {
        let mut params = Vec::new();
        if self.style.bold {
            params.push("1".to_owned());
        }
        if self.style.dim {
            params.push("2".to_owned());
        }
        if self.style.italic {
            params.push("3".to_owned());
        }
        if self.style.underline {
            params.push(match self.style.underline_style {
                UnderlineStyle::Single => "4".to_owned(),
                UnderlineStyle::Double => "21".to_owned(),
                UnderlineStyle::Curly => "4:3".to_owned(),
                UnderlineStyle::Dotted => "4:4".to_owned(),
                UnderlineStyle::Dashed => "4:5".to_owned(),
            });
        }
        if self.style.blink {
            params.push("5".to_owned());
        }
        if self.style.inverse {
            params.push("7".to_owned());
        }
        if self.style.hidden {
            params.push("8".to_owned());
        }
        if self.style.strikethrough {
            params.push("9".to_owned());
        }
        if self.style.framed {
            params.push("51".to_owned());
        }
        if self.style.encircled {
            params.push("52".to_owned());
        }
        if self.style.overline {
            params.push("53".to_owned());
        }
        push_sgr_color_parameters(&mut params, 30, 90, 38, self.style.foreground);
        push_sgr_color_parameters(&mut params, 40, 100, 48, self.style.background);
        if let Some(color) = self.active_underline_color() {
            push_sgr_extended_color_parameter(&mut params, 58, color);
        }

        if params.is_empty() {
            params.push("0".to_owned());
        }
        params
    }

    fn active_underline_color(&self) -> Option<Color> {
        if self.style.underline_color_id == 0 {
            return None;
        }
        self.underline_colors
            .get(usize::from(self.style.underline_color_id - 1))
            .copied()
            .filter(|color| *color != Color::Default)
    }

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

    pub(super) fn report_decrqss(&mut self, request: &[u8]) {
        match request {
            b"m" => self.pending_response_bytes.extend_from_slice(
                format!("\x1bP1$r{}m\x1b\\", self.active_sgr_parameters().join(";")).as_bytes(),
            ),
            b"r" => self.pending_response_bytes.extend_from_slice(
                format!(
                    "\x1bP1$r{};{}r\x1b\\",
                    self.scroll_top + 1,
                    self.scroll_bottom + 1
                )
                .as_bytes(),
            ),
            b" q" => self.pending_response_bytes.extend_from_slice(
                format!("\x1bP1$r{} q\x1b\\", self.cursor_shape_parameter()).as_bytes(),
            ),
            _ => self
                .pending_response_bytes
                .extend_from_slice(b"\x1bP0$r\x1b\\"),
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
