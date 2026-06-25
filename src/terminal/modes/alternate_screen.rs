use crate::grid::Grid;

use super::Terminal;
use crate::terminal::params::default_tab_stops;
use crate::terminal::state::{CharacterSet, SavedScreen};

impl Terminal {
    pub(super) fn enter_alternate_screen(&mut self) {
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

    pub(super) fn leave_alternate_screen(&mut self) {
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
