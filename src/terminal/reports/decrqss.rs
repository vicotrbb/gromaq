use crate::cell::{Color, UnderlineStyle};

use super::super::params::{push_sgr_color_parameters, push_sgr_extended_color_parameter};
use super::super::{CursorShape, Terminal};

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

    pub(in crate::terminal) fn report_decrqss(&mut self, request: &[u8]) {
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
}
