//! CSI dispatch for parser callbacks.

use vte::Params;

use crate::terminal::Terminal;
use crate::terminal::params::{first_value, first_values};

impl Terminal {
    pub(super) fn dispatch_csi(
        &mut self,
        params: &Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
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
            'H' | 'f' => self.set_csi_cursor_position(params),
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
            'r' => self.set_csi_scroll_region(params),
            's' if intermediates == b"?" => self.save_private_modes(first_values(params)),
            's' => self.save_cursor(),
            't' if intermediates.is_empty() => self.dispatch_window_manipulation(params),
            'u' => self.restore_cursor(),
            'x' if intermediates.is_empty() => self.report_terminal_parameters(first),
            'h' if intermediates.is_empty() => self.set_ansi_modes(params, true),
            'l' if intermediates.is_empty() => self.set_ansi_modes(params, false),
            'h' if intermediates == b"?" => self.set_dec_private_modes(params, true),
            'l' if intermediates == b"?" => self.set_dec_private_modes(params, false),
            _ => {}
        }
    }

    fn set_csi_cursor_position(&mut self, params: &Params) {
        let row = first_value(params, 0).unwrap_or(1);
        let col = first_value(params, 1).unwrap_or(1);
        self.set_cursor_position(row, col);
    }

    fn set_csi_scroll_region(&mut self, params: &Params) {
        let top = match first_value(params, 0) {
            Some(0) | None => 1,
            Some(value) => value,
        };
        let bottom = match first_value(params, 1) {
            Some(0) | None => self.config.rows,
            Some(value) => value,
        };
        self.set_scroll_region(top, bottom);
    }

    fn dispatch_window_manipulation(&mut self, params: &Params) {
        let operation = first_value(params, 0).unwrap_or(0);
        let target = first_value(params, 1).unwrap_or(0);
        self.report_window_manipulation(operation, target);
    }

    fn set_ansi_modes(&mut self, params: &Params, enabled: bool) {
        for mode in first_values(params) {
            self.set_mode(mode, enabled);
        }
    }

    fn set_dec_private_modes(&mut self, params: &Params, enabled: bool) {
        for mode in first_values(params) {
            self.set_private_mode(mode, enabled);
        }
    }
}
