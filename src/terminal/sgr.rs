//! SGR color and text attribute state transitions.

use vte::Params;

use crate::cell::{Color, Style, UnderlineStyle};

use super::params::{
    apply_grouped_sgr_param, grouped_extended_color, is_invalid_grouped_extended_color_param,
    parse_extended_color,
};
use super::width::metadata_id_for_index;
use super::{MAX_METADATA_IDS, Terminal};

const MAX_UNDERLINE_COLORS: usize = MAX_METADATA_IDS;

impl Terminal {
    pub(super) fn apply_sgr(&mut self, params: &Params) {
        if params.is_empty() {
            self.style = Style::default();
            return;
        }
        let mut flattened = Vec::new();
        for param in params.iter() {
            if param.is_empty() {
                flattened.push(0);
                continue;
            }
            if apply_grouped_sgr_param(&mut self.style, param) {
                continue;
            }
            if self.apply_grouped_extended_color_param(param) {
                continue;
            }
            if is_invalid_grouped_extended_color_param(param) {
                continue;
            }
            flattened.extend(param.iter().copied());
        }
        self.apply_flat_sgr(flattened);
    }

    fn apply_flat_sgr(&mut self, params: Vec<u16>) {
        let mut iter = params.into_iter().peekable();
        while let Some(param) = iter.next() {
            match param {
                0 => self.style = Style::default(),
                1 => self.style.bold = true,
                2 => self.style.dim = true,
                3 => self.style.italic = true,
                4 => self.style.underline = true,
                21 => {
                    self.style.underline = true;
                    self.style.underline_style = UnderlineStyle::Double;
                }
                5 | 6 => self.style.blink = true,
                7 => self.style.inverse = true,
                8 => self.style.hidden = true,
                9 => self.style.strikethrough = true,
                22 => {
                    self.style.bold = false;
                    self.style.dim = false;
                }
                23 => self.style.italic = false,
                24 => {
                    self.style.underline = false;
                    self.style.underline_style = UnderlineStyle::Single;
                }
                25 => self.style.blink = false,
                27 => self.style.inverse = false,
                28 => self.style.hidden = false,
                29 => self.style.strikethrough = false,
                51 => {
                    self.style.framed = true;
                    self.style.encircled = false;
                }
                52 => {
                    self.style.framed = false;
                    self.style.encircled = true;
                }
                53 => self.style.overline = true,
                54 => {
                    self.style.framed = false;
                    self.style.encircled = false;
                }
                55 => self.style.overline = false,
                59 => self.style.underline_color_id = 0,
                30..=37 => self.style.foreground = Color::Ansi((param - 30) as u8),
                39 => self.style.foreground = Color::Default,
                40..=47 => self.style.background = Color::Ansi((param - 40) as u8),
                49 => self.style.background = Color::Default,
                90..=97 => self.style.foreground = Color::Ansi((param - 90 + 8) as u8),
                100..=107 => self.style.background = Color::Ansi((param - 100 + 8) as u8),
                38 | 48 | 58 => {
                    if let Some(color) = parse_extended_color(&mut iter) {
                        self.apply_extended_color_target(param, color);
                    }
                }
                _ => {}
            }
        }
    }

    fn apply_grouped_extended_color_param(&mut self, param: &[u16]) -> bool {
        let Some((target, color)) = grouped_extended_color(param) else {
            return false;
        };
        self.apply_extended_color_target(target, color)
    }

    fn apply_extended_color_target(&mut self, target: u16, color: Color) -> bool {
        match target {
            38 => self.style.foreground = color,
            48 => self.style.background = color,
            58 => self.style.underline_color_id = self.intern_underline_color(color),
            _ => return false,
        }
        true
    }

    fn intern_underline_color(&mut self, color: Color) -> u16 {
        if color == Color::Default {
            return 0;
        }
        if let Some(index) = self
            .underline_colors
            .iter()
            .position(|existing| *existing == color)
        {
            return metadata_id_for_index(index);
        }
        if self.underline_colors.len() == MAX_UNDERLINE_COLORS {
            return 0;
        }
        self.underline_colors.push(color);
        metadata_id_for_index(self.underline_colors.len() - 1)
    }
}
