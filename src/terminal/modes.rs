use super::Terminal;
use super::state::SavedCursorState;

mod alternate_screen;

impl Terminal {
    pub(super) fn save_dec_cursor(&mut self) {
        self.saved_dec_cursor = Some(SavedCursorState {
            cursor: self.cursor,
            wrap_pending: self.wrap_pending,
            style: self.style,
            origin_mode: self.origin_mode,
            g0_dec_special_graphics: self.g0_dec_special_graphics,
            g1_dec_special_graphics: self.g1_dec_special_graphics,
            active_charset: self.active_charset,
        });
    }

    pub(super) fn restore_dec_cursor(&mut self) {
        if let Some(mut saved) = self.saved_dec_cursor {
            saved.cursor.clamp_to(self.config.cols, self.config.rows);
            self.cursor = saved.cursor;
            self.wrap_pending = saved.wrap_pending;
            self.style = saved.style;
            self.origin_mode = saved.origin_mode;
            self.g0_dec_special_graphics = saved.g0_dec_special_graphics;
            self.g1_dec_special_graphics = saved.g1_dec_special_graphics;
            self.active_charset = saved.active_charset;
        }
    }

    pub(super) fn set_private_mode(&mut self, mode: u16, enabled: bool) {
        match mode {
            1 => self.application_cursor_keys = enabled,
            6 => {
                self.origin_mode = enabled;
                self.cursor.row = if enabled { self.scroll_top } else { 0 };
                self.cursor.col = 0;
                self.wrap_pending = false;
            }
            7 => {
                self.auto_wrap = enabled;
                if !enabled {
                    self.wrap_pending = false;
                }
            }
            12 => self.cursor.blinking = enabled,
            25 => self.cursor.visible = enabled,
            66 => self.application_keypad = enabled,
            9 => self.mouse.set_x10_reporting(enabled),
            47 | 1047 if enabled => self.enter_alternate_screen(),
            47 | 1047 => self.leave_alternate_screen(),
            1048 if enabled => self.save_dec_cursor(),
            1048 => self.restore_dec_cursor(),
            1049 if enabled => {
                if self.saved_primary.is_none() {
                    self.save_dec_cursor();
                }
                self.enter_alternate_screen();
            }
            1049 => {
                let was_in_alternate_screen = self.saved_primary.is_some();
                self.leave_alternate_screen();
                if was_in_alternate_screen {
                    self.restore_dec_cursor();
                }
            }
            1000 => self.mouse.set_button_reporting(enabled),
            1002 => self.mouse.set_button_motion_reporting(enabled),
            1003 => self.mouse.set_any_motion_reporting(enabled),
            1004 => self.focus_event_reporting = enabled,
            1006 => self.mouse.set_sgr_protocol(enabled),
            2004 => self.bracketed_paste = enabled,
            _ => {}
        }
    }

    pub(super) fn private_mode_state(&self, mode: u16) -> Option<bool> {
        match mode {
            1 => Some(self.application_cursor_keys),
            6 => Some(self.origin_mode),
            7 => Some(self.auto_wrap),
            12 => Some(self.cursor.blinking),
            25 => Some(self.cursor.visible),
            66 => Some(self.application_keypad),
            9 => Some(self.mouse.x10_reporting_enabled()),
            47 | 1047 | 1049 => Some(self.saved_primary.is_some()),
            1000 => Some(self.mouse.button_reporting_enabled()),
            1002 => Some(self.mouse.button_motion_reporting_enabled()),
            1003 => Some(self.mouse.any_motion_reporting_enabled()),
            1004 => Some(self.focus_event_reporting),
            1006 => Some(self.mouse.sgr_protocol_enabled()),
            2004 => Some(self.bracketed_paste),
            _ => None,
        }
    }

    pub(super) fn save_private_modes(&mut self, modes: impl IntoIterator<Item = u16>) {
        for mode in modes {
            let Some(enabled) = self.private_mode_state(mode) else {
                continue;
            };
            if let Some((_, saved)) = self
                .saved_private_modes
                .iter_mut()
                .find(|(saved_mode, _)| *saved_mode == mode)
            {
                *saved = enabled;
            } else {
                self.saved_private_modes.push((mode, enabled));
            }
        }
    }

    pub(super) fn restore_private_modes(&mut self, modes: impl IntoIterator<Item = u16>) {
        let restores: Vec<(u16, bool)> = modes
            .into_iter()
            .filter_map(|mode| {
                self.saved_private_modes
                    .iter()
                    .find(|(saved_mode, _)| *saved_mode == mode)
                    .copied()
            })
            .collect();
        for (mode, enabled) in restores {
            self.set_private_mode(mode, enabled);
        }
    }

    pub(super) fn set_mode(&mut self, mode: u16, enabled: bool) {
        match mode {
            4 => self.insert_mode = enabled,
            20 => self.linefeed_newline_mode = enabled,
            _ => {}
        }
    }
}
