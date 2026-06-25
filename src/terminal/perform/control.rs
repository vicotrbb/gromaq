//! Control-character dispatch for parser callbacks.

use crate::terminal::Terminal;
use crate::terminal::state::CharacterSet;

impl Terminal {
    pub(super) fn execute_control(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0b | 0x0c => self.execute_line_feed_control(),
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

    fn execute_line_feed_control(&mut self) {
        if let Some(hard_break) = self.hard_breaks.get_mut(usize::from(self.cursor.row)) {
            *hard_break = true;
        }
        self.wrap_pending = false;
        self.line_feed();
        if self.linefeed_newline_mode {
            self.carriage_return();
        }
    }
}
