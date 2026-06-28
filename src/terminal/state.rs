//! Private terminal state records shared by parser and buffer operations.

use crate::cell::Style;
use crate::grid::Grid;

use super::CursorShape;

#[derive(Debug, Clone, Copy)]
pub(super) struct Cursor {
    pub(super) row: u16,
    pub(super) col: u16,
    pub(super) visible: bool,
    pub(super) shape: CursorShape,
    pub(super) blinking: bool,
}

impl Cursor {
    pub(super) fn clamp_to(&mut self, cols: u16, rows: u16) {
        self.row = self.row.min(rows - 1);
        self.col = self.col.min(cols - 1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CharacterSet {
    G0,
    G1,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SavedCursorState {
    pub(super) cursor: Cursor,
    pub(super) wrap_pending: bool,
    pub(super) style: Style,
    pub(super) origin_mode: bool,
    pub(super) g0_dec_special_graphics: bool,
    pub(super) g1_dec_special_graphics: bool,
    pub(super) active_charset: CharacterSet,
}

#[derive(Debug, Clone)]
pub(super) struct SavedScreen {
    pub(super) grid: Grid,
    pub(super) cursor: Cursor,
    pub(super) hard_breaks: Vec<bool>,
    pub(super) tab_stops: Vec<bool>,
    pub(super) wrap_pending: bool,
    pub(super) auto_wrap: bool,
    pub(super) origin_mode: bool,
    pub(super) application_cursor_keys: bool,
    pub(super) application_keypad: bool,
    pub(super) focus_event_reporting: bool,
    pub(super) insert_mode: bool,
    pub(super) linefeed_newline_mode: bool,
    pub(super) g0_dec_special_graphics: bool,
    pub(super) g1_dec_special_graphics: bool,
    pub(super) active_charset: CharacterSet,
    pub(super) scroll_top: u16,
    pub(super) scroll_bottom: u16,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DirtyRun {
    pub(super) row: u16,
    pub(super) col_start: u16,
    pub(super) col_end: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DcsHandler {
    Decrqss,
}
