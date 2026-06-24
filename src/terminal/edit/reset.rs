//! Terminal reset and cursor save/restore helpers.

use crate::cell::Style;
use crate::dirty::DirtyTracker;
use crate::grid::Grid;
use crate::mouse::MouseReportState;
use crate::scrollback::Scrollback;

use super::super::params::default_tab_stops;
use super::super::state::{CharacterSet, Cursor, SavedCursorState};
use super::super::{Terminal, initial_cursor};

impl Terminal {
    pub(in crate::terminal) fn save_cursor(&mut self) {
        self.saved_cursor = Some(self.cursor);
    }

    pub(in crate::terminal) fn restore_cursor(&mut self) {
        if let Some(saved) = self.saved_cursor {
            self.cursor = saved;
        }
    }

    #[cold]
    #[inline(never)]
    pub(in crate::terminal) fn soft_reset(&mut self) {
        self.wrap_pending = false;
        self.auto_wrap = false;
        self.origin_mode = false;
        self.application_cursor_keys = false;
        self.application_keypad = false;
        self.insert_mode = false;
        self.linefeed_newline_mode = false;
        self.cursor.visible = true;
        self.g0_dec_special_graphics = false;
        self.g1_dec_special_graphics = false;
        self.active_charset = CharacterSet::G0;
        self.scroll_top = 0;
        self.scroll_bottom = self.config.rows - 1;
        self.style = Style::default();
        self.saved_dec_cursor = Some(SavedCursorState {
            cursor: Cursor {
                row: 0,
                col: 0,
                visible: true,
                shape: self.config.cursor_shape,
                blinking: self.config.cursor_blinking,
            },
            style: Style::default(),
            g0_dec_special_graphics: false,
            g1_dec_special_graphics: false,
            active_charset: CharacterSet::G0,
        });
    }

    pub(in crate::terminal) fn reset_to_initial_state(&mut self) {
        self.flush_dirty_run();
        self.grid = Grid::new(self.config.cols, self.config.rows);
        self.hard_breaks = vec![false; usize::from(self.config.rows)];
        self.tab_stops = default_tab_stops(self.config.cols);
        self.scrollback = Scrollback::new(self.config.scrollback_limit);
        self.cursor = initial_cursor(&self.config);
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
        self.scroll_top = 0;
        self.scroll_bottom = self.config.rows - 1;
        self.saved_cursor = None;
        self.saved_dec_cursor = None;
        self.saved_primary = None;
        self.scrollback_view_offset = 0;
        self.saved_private_modes.clear();
        self.selection = None;
        self.dirty = DirtyTracker::default();
        self.dirty_run = None;
        self.mouse = MouseReportState::default();
        self.title = None;
        self.icon_label = None;
        self.clipboard_text = None;
        self.hyperlinks.clear();
        self.current_hyperlink_id = 0;
        self.underline_colors.clear();
        self.bracketed_paste = false;
        self.dcs_handler = None;
        self.dcs_payload_overflowed = false;
        self.dcs_payload.clear();
        self.pending_response_bytes.clear();
        self.style = Style::default();
        self.last_printable_char = None;
        self.dirty.mark_viewport(self.config.rows, self.config.cols);
        self.perf.dirty_cells += u64::from(self.config.rows) * u64::from(self.config.cols);
    }
}
