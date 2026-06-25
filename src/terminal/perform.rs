//! `vte::Perform` adapter for terminal parser callbacks.

use vte::{Params, Perform};

use super::Terminal;
use super::params::{first_value, first_values};
use super::state::CharacterSet;
use super::width::map_dec_special_graphics;

impl Perform for Terminal {
    fn hook(&mut self, _params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        self.start_dcs_handler(_params, intermediates, ignore, action);
    }

    fn put(&mut self, byte: u8) {
        self.push_dcs_byte(byte);
    }

    fn unhook(&mut self) {
        self.finish_dcs_handler();
    }

    fn print(&mut self, c: char) {
        let dec_special_graphics = match self.active_charset {
            CharacterSet::G0 => self.g0_dec_special_graphics,
            CharacterSet::G1 => self.g1_dec_special_graphics,
        };
        let ch = if dec_special_graphics {
            map_dec_special_graphics(c)
        } else {
            c
        };
        self.put_char(ch);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0b | 0x0c => {
                if let Some(hard_break) = self.hard_breaks.get_mut(usize::from(self.cursor.row)) {
                    *hard_break = true;
                }
                self.wrap_pending = false;
                self.line_feed();
                if self.linefeed_newline_mode {
                    self.carriage_return();
                }
            }
            b'\r' => self.carriage_return(),
            0x08 => self.backspace(),
            b'\t' => self.horizontal_tab(),
            0x0e => self.active_charset = CharacterSet::G1,
            0x0f => self.active_charset = CharacterSet::G0,
            0x84 => self.index(),
            0x85 => self.next_line(),
            0x88 => self.set_horizontal_tab_stop(),
            0x8d => self.reverse_index(),
            0x9a => self.report_decid(),
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        if ignore {
            return;
        }
        let first = first_value(params, 0).unwrap_or(0);
        let count = if first == 0 { 1 } else { first };
        match action {
            'A' => self.move_cursor_up(count),
            'B' => self.move_cursor_down(count),
            'C' => self.move_cursor_right(count),
            'D' => self.move_cursor_left(count),
            'E' => self.move_cursor_next_line(count),
            'F' => self.move_cursor_previous_line(count),
            'I' => self.move_cursor_forward_tabs(count),
            'Z' => self.move_cursor_backward_tabs(count),
            '@' => self.insert_blank_chars(count),
            'b' => self.repeat_last_printable_char(count),
            'P' => self.delete_chars(count),
            'X' => self.erase_chars(count),
            'c' if intermediates.is_empty() => self.report_primary_device_attributes(first),
            'c' if intermediates == b">" => self.report_secondary_device_attributes(first),
            'L' => self.insert_blank_lines(count),
            'M' => self.delete_lines(count),
            'S' => self.scroll_viewport_up(count),
            'T' => self.scroll_viewport_down(count),
            '^' => self.scroll_viewport_down(count),
            '`' => self.set_cursor_col(count),
            'a' => self.move_cursor_right(count),
            'H' | 'f' => {
                let row = first_value(params, 0).unwrap_or(1);
                let col = first_value(params, 1).unwrap_or(1);
                self.set_cursor_position(row, col);
            }
            'G' => self.set_cursor_col(count),
            'g' => self.clear_tab_stop(first),
            'd' => self.set_cursor_row(count),
            'e' => self.move_cursor_down(count),
            'J' => self.erase_display(first),
            'K' => self.erase_line(first),
            'm' => self.apply_sgr(params),
            'n' if intermediates.is_empty() => self.report_device_status(first),
            'n' if intermediates == b"?" => self.report_private_device_status(first),
            'p' if intermediates == b"$" => self.report_mode_state(false, first),
            'p' if intermediates == b"?$" => self.report_mode_state(true, first),
            'p' if intermediates == b"!" => self.soft_reset(),
            'q' if intermediates == b" " => self.set_cursor_shape(first),
            'r' if intermediates == b"?" => self.restore_private_modes(first_values(params)),
            'r' => {
                let top = first_value(params, 0).unwrap_or(1);
                let bottom = first_value(params, 1).unwrap_or(self.config.rows);
                self.set_scroll_region(top, bottom);
            }
            's' if intermediates == b"?" => self.save_private_modes(first_values(params)),
            's' => self.save_cursor(),
            't' if intermediates.is_empty() => self.report_window_manipulation(first),
            'u' => self.restore_cursor(),
            'x' if intermediates.is_empty() => self.report_terminal_parameters(first),
            'h' if intermediates.is_empty() => {
                for mode in first_values(params) {
                    self.set_mode(mode, true);
                }
            }
            'l' if intermediates.is_empty() => {
                for mode in first_values(params) {
                    self.set_mode(mode, false);
                }
            }
            'h' if intermediates == b"?" => {
                for mode in first_values(params) {
                    self.set_private_mode(mode, true);
                }
            }
            'l' if intermediates == b"?" => {
                for mode in first_values(params) {
                    self.set_private_mode(mode, false);
                }
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        if ignore {
            return;
        }
        match (intermediates, byte) {
            (b"(", b'0') => self.g0_dec_special_graphics = true,
            (b"(", b'B') => self.g0_dec_special_graphics = false,
            (b"(", b'A') => self.g0_dec_special_graphics = false,
            (b"(", b'U') => self.g0_dec_special_graphics = false,
            (b")", b'0') => self.g1_dec_special_graphics = true,
            (b")", b'B') => self.g1_dec_special_graphics = false,
            (b")", b'A') => self.g1_dec_special_graphics = false,
            (b")", b'U') => self.g1_dec_special_graphics = false,
            (b"", b'D') => self.index(),
            (b"", b'E') => self.next_line(),
            (b"", b'H') => self.set_horizontal_tab_stop(),
            (b"", b'M') => self.reverse_index(),
            (b"", b'=') => self.application_keypad = true,
            (b"", b'>') => self.application_keypad = false,
            (b"", b'7') => self.save_dec_cursor(),
            (b"", b'8') => self.restore_dec_cursor(),
            (b"", b'Z') => self.report_decid(),
            (b"#", b'8') => self.screen_alignment_test(),
            (b"", b'c') => self.reset_to_initial_state(),
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        self.dispatch_osc(params);
    }
}
