use crate::grid::Grid;

use super::Terminal;
use super::params::default_tab_stops;
use super::state::{CharacterSet, SavedCursorState, SavedScreen};

impl Terminal {
    pub(super) fn save_dec_cursor(&mut self) {
        self.saved_dec_cursor = Some(SavedCursorState {
            cursor: self.cursor,
            style: self.style,
            g0_dec_special_graphics: self.g0_dec_special_graphics,
            g1_dec_special_graphics: self.g1_dec_special_graphics,
            active_charset: self.active_charset,
        });
    }

    pub(super) fn restore_dec_cursor(&mut self) {
        if let Some(saved) = self.saved_dec_cursor {
            self.cursor = saved.cursor;
            self.style = saved.style;
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

    fn enter_alternate_screen(&mut self) {
        if self.saved_primary.is_some() {
            return;
        }
        self.flush_dirty_run();
        self.saved_primary = Some(SavedScreen {
            grid: self.grid.clone(),
            cursor: self.cursor,
            hard_breaks: self.hard_breaks.clone(),
            tab_stops: self.tab_stops.clone(),
            wrap_pending: self.wrap_pending,
            auto_wrap: self.auto_wrap,
            origin_mode: self.origin_mode,
            application_cursor_keys: self.application_cursor_keys,
            application_keypad: self.application_keypad,
            focus_event_reporting: self.focus_event_reporting,
            insert_mode: self.insert_mode,
            linefeed_newline_mode: self.linefeed_newline_mode,
            g0_dec_special_graphics: self.g0_dec_special_graphics,
            g1_dec_special_graphics: self.g1_dec_special_graphics,
            active_charset: self.active_charset,
            scroll_top: self.scroll_top,
            scroll_bottom: self.scroll_bottom,
        });
        self.grid = Grid::new(self.config.cols, self.config.rows);
        self.hard_breaks = vec![false; usize::from(self.config.rows)];
        self.tab_stops = default_tab_stops(self.config.cols);
        self.scroll_top = 0;
        self.scroll_bottom = self.config.rows - 1;
        self.cursor.row = 0;
        self.cursor.col = 0;
        self.wrap_pending = false;
        self.auto_wrap = true;
        self.origin_mode = false;
        self.application_cursor_keys = false;
        self.application_keypad = false;
        self.focus_event_reporting = false;
        self.insert_mode = false;
        self.linefeed_newline_mode = false;
        self.g0_dec_special_graphics = false;
        self.g1_dec_special_graphics = false;
        self.active_charset = CharacterSet::G0;
        self.scrollback_view_offset = 0;
        self.selection = None;
        self.dirty.mark_viewport(self.config.rows, self.config.cols);
    }

    fn leave_alternate_screen(&mut self) {
        if let Some(saved) = self.saved_primary.take() {
            self.flush_dirty_run();
            self.grid = saved.grid;
            self.cursor = saved.cursor;
            self.hard_breaks = saved.hard_breaks;
            self.tab_stops = saved.tab_stops;
            self.wrap_pending = saved.wrap_pending;
            self.auto_wrap = saved.auto_wrap;
            self.origin_mode = saved.origin_mode;
            self.application_cursor_keys = saved.application_cursor_keys;
            self.application_keypad = saved.application_keypad;
            self.focus_event_reporting = saved.focus_event_reporting;
            self.insert_mode = saved.insert_mode;
            self.linefeed_newline_mode = saved.linefeed_newline_mode;
            self.g0_dec_special_graphics = saved.g0_dec_special_graphics;
            self.g1_dec_special_graphics = saved.g1_dec_special_graphics;
            self.active_charset = saved.active_charset;
            self.scroll_top = saved.scroll_top;
            self.scroll_bottom = saved.scroll_bottom;
            self.selection = None;
            self.dirty.mark_viewport(self.config.rows, self.config.cols);
        }
    }
}
