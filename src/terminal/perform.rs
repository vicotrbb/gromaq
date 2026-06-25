//! `vte::Perform` adapter for terminal parser callbacks.

mod control;
mod csi;

use vte::{Params, Perform};

use super::Terminal;
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
        self.execute_control(byte);
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        self.dispatch_csi(params, intermediates, ignore, action);
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
