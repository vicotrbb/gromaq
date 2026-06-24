//! VT editing, cursor movement, scrolling, and dirty-region helpers.

use crate::cell::{Cell, Style};
use crate::dirty::{DirtyRegion, DirtyTracker};
use crate::grid::Grid;
use crate::mouse::MouseReportState;
use crate::scrollback::Scrollback;

use super::params::default_tab_stops;
use super::reflow;
use super::state::{CharacterSet, Cursor, DirtyRun, SavedCursorState};
use super::{CursorShape, Terminal};

impl Terminal {
    pub(super) fn carriage_return(&mut self) {
        self.wrap_pending = false;
        self.cursor.col = 0;
    }

    pub(super) fn backspace(&mut self) {
        self.wrap_pending = false;
        self.cursor.col = self.cursor.col.saturating_sub(1);
    }

    pub(super) fn horizontal_tab(&mut self) {
        self.wrap_pending = false;
        self.cursor.col = self
            .tab_stops
            .iter()
            .enumerate()
            .skip(usize::from(self.cursor.col + 1))
            .find_map(|(col, enabled)| enabled.then_some(col as u16))
            .unwrap_or(self.config.cols - 1);
    }

    pub(super) fn set_horizontal_tab_stop(&mut self) {
        if let Some(tab_stop) = self.tab_stops.get_mut(usize::from(self.cursor.col)) {
            *tab_stop = true;
        }
    }

    pub(super) fn clear_tab_stop(&mut self, mode: u16) {
        match mode {
            0 => {
                if let Some(tab_stop) = self.tab_stops.get_mut(usize::from(self.cursor.col)) {
                    *tab_stop = false;
                }
            }
            3 => self.tab_stops.fill(false),
            _ => {}
        }
    }

    pub(super) fn move_cursor_forward_tabs(&mut self, count: u16) {
        for _ in 0..count {
            self.horizontal_tab();
        }
    }

    pub(super) fn move_cursor_backward_tabs(&mut self, count: u16) {
        self.wrap_pending = false;
        for _ in 0..count {
            self.cursor.col = self
                .tab_stops
                .iter()
                .enumerate()
                .take(usize::from(self.cursor.col))
                .rev()
                .find_map(|(col, enabled)| enabled.then_some(col as u16))
                .unwrap_or(0);
        }
    }

    pub(super) fn move_cursor_left(&mut self, count: u16) {
        self.wrap_pending = false;
        self.cursor.col = self.cursor.col.saturating_sub(count);
    }

    pub(super) fn move_cursor_right(&mut self, count: u16) {
        self.wrap_pending = false;
        self.cursor.col = (self.cursor.col + count).min(self.config.cols - 1);
    }

    pub(super) fn move_cursor_up(&mut self, count: u16) {
        self.wrap_pending = false;
        let (top, _) = self.vertical_cursor_bounds();
        self.cursor.row = self.cursor.row.saturating_sub(count).max(top);
    }

    pub(super) fn move_cursor_down(&mut self, count: u16) {
        self.wrap_pending = false;
        let (_, bottom) = self.vertical_cursor_bounds();
        self.cursor.row = self.cursor.row.saturating_add(count).min(bottom);
    }

    pub(super) fn move_cursor_next_line(&mut self, count: u16) {
        self.move_cursor_down(count);
        self.cursor.col = 0;
    }

    pub(super) fn move_cursor_previous_line(&mut self, count: u16) {
        self.move_cursor_up(count);
        self.cursor.col = 0;
    }

    pub(super) fn set_cursor_position(&mut self, row: u16, col: u16) {
        self.wrap_pending = false;
        self.cursor.row = self.absolute_cursor_row(row);
        self.cursor.col = col.saturating_sub(1).min(self.config.cols - 1);
    }

    pub(super) fn set_cursor_col(&mut self, col: u16) {
        self.wrap_pending = false;
        self.cursor.col = col.saturating_sub(1).min(self.config.cols - 1);
    }

    pub(super) fn set_cursor_row(&mut self, row: u16) {
        self.wrap_pending = false;
        self.cursor.row = self.absolute_cursor_row(row);
    }

    pub(super) fn set_cursor_shape(&mut self, shape: u16) {
        let Some((shape, blinking)) = (match shape {
            0 | 1 => Some((CursorShape::Block, true)),
            2 => Some((CursorShape::Block, false)),
            3 => Some((CursorShape::Underline, true)),
            4 => Some((CursorShape::Underline, false)),
            5 => Some((CursorShape::Bar, true)),
            6 => Some((CursorShape::Bar, false)),
            _ => None,
        }) else {
            return;
        };
        self.cursor.shape = shape;
        self.cursor.blinking = blinking;
    }

    fn absolute_cursor_row(&self, row: u16) -> u16 {
        let row = row.saturating_sub(1);
        if self.origin_mode {
            self.scroll_top.saturating_add(row).min(self.scroll_bottom)
        } else {
            row.min(self.config.rows - 1)
        }
    }

    fn vertical_cursor_bounds(&self) -> (u16, u16) {
        if self.origin_mode {
            (self.scroll_top, self.scroll_bottom)
        } else {
            (0, self.config.rows - 1)
        }
    }

    pub(super) fn erase_line(&mut self, mode: u16) {
        self.flush_dirty_run();
        match mode {
            0 => {
                for col in self.cursor.col..self.config.cols {
                    self.grid.clear_cell(self.cursor.row, col, self.style);
                }
                let cols = self.config.cols - self.cursor.col;
                self.dirty.mark_span(self.cursor.row, self.cursor.col, cols);
                self.perf.dirty_cells += u64::from(cols);
                if self.cursor.col == 0 {
                    self.hard_breaks[usize::from(self.cursor.row)] = false;
                }
            }
            1 => {
                for col in 0..=self.cursor.col {
                    self.grid.clear_cell(self.cursor.row, col, self.style);
                }
                let cols = self.cursor.col + 1;
                self.dirty.mark_span(self.cursor.row, 0, cols);
                self.perf.dirty_cells += u64::from(cols);
                if self.cursor.col + 1 == self.config.cols {
                    self.hard_breaks[usize::from(self.cursor.row)] = false;
                }
            }
            2 => {
                self.grid.clear_row(self.cursor.row, self.style);
                self.hard_breaks[usize::from(self.cursor.row)] = false;
                self.dirty.mark_span(self.cursor.row, 0, self.config.cols);
                self.perf.dirty_cells += u64::from(self.config.cols);
            }
            _ => {}
        }
    }

    pub(super) fn erase_display(&mut self, mode: u16) {
        self.flush_dirty_run();
        match mode {
            0 => {
                for row in self.cursor.row..self.config.rows {
                    let start_col = if row == self.cursor.row {
                        self.cursor.col
                    } else {
                        0
                    };
                    for col in start_col..self.config.cols {
                        self.grid.clear_cell(row, col, self.style);
                    }
                    let cols = self.config.cols - start_col;
                    self.dirty.mark_span(row, start_col, cols);
                    self.perf.dirty_cells += u64::from(cols);
                    if start_col == 0 {
                        self.hard_breaks[usize::from(row)] = false;
                    }
                }
            }
            1 => {
                for row in 0..=self.cursor.row {
                    let end_col = if row == self.cursor.row {
                        self.cursor.col
                    } else {
                        self.config.cols - 1
                    };
                    for col in 0..=end_col {
                        self.grid.clear_cell(row, col, self.style);
                    }
                    let cols = end_col + 1;
                    self.dirty.mark_span(row, 0, cols);
                    self.perf.dirty_cells += u64::from(cols);
                    if end_col + 1 == self.config.cols {
                        self.hard_breaks[usize::from(row)] = false;
                    }
                }
            }
            2 => {
                for row in 0..self.config.rows {
                    self.grid.clear_row(row, self.style);
                }
                self.hard_breaks.fill(false);
                self.wrap_pending = false;
                self.perf.dirty_cells += u64::from(self.config.cols) * u64::from(self.config.rows);
                self.dirty.mark_viewport(self.config.rows, self.config.cols);
            }
            3 => self.scrollback.clear(),
            _ => {}
        }
    }

    #[cold]
    #[inline(never)]
    pub(super) fn screen_alignment_test(&mut self) {
        self.flush_dirty_run();
        self.wrap_pending = false;
        for row in 0..self.config.rows {
            for col in 0..self.config.cols {
                *self.grid.cell_mut(row, col) = Cell {
                    text: "E".to_owned(),
                    style: self.style,
                    hyperlink_id: 0,
                    is_wide_leading: false,
                    is_wide_trailing: false,
                };
            }
            self.hard_breaks[usize::from(row)] = false;
        }
        self.perf.dirty_cells += u64::from(self.config.cols) * u64::from(self.config.rows);
        self.dirty.mark_viewport(self.config.rows, self.config.cols);
    }

    pub(super) fn insert_blank_chars(&mut self, count: u16) {
        self.flush_dirty_run();
        self.grid
            .insert_blank_cells(self.cursor.row, self.cursor.col, count, self.style);
        let repaired = self
            .grid
            .repair_wide_cells_in_row(self.cursor.row, self.style);
        self.mark_edit_span_dirty(self.cursor.col, self.config.cols, repaired);
    }

    pub(super) fn delete_chars(&mut self, count: u16) {
        self.flush_dirty_run();
        self.grid
            .delete_cells(self.cursor.row, self.cursor.col, count, self.style);
        let repaired = self
            .grid
            .repair_wide_cells_in_row(self.cursor.row, self.style);
        self.mark_edit_span_dirty(self.cursor.col, self.config.cols, repaired);
    }

    pub(super) fn erase_chars(&mut self, count: u16) {
        self.flush_dirty_run();
        let count = count.min(self.config.cols - self.cursor.col);
        for col in self.cursor.col..self.cursor.col + count {
            self.grid.clear_cell(self.cursor.row, col, self.style);
        }
        let repaired = self
            .grid
            .repair_wide_cells_in_row(self.cursor.row, self.style);
        self.mark_edit_span_dirty(self.cursor.col, self.cursor.col + count, repaired);
    }

    fn mark_edit_span_dirty(
        &mut self,
        edit_start: u16,
        edit_end: u16,
        repaired: Option<(u16, u16)>,
    ) {
        let (start, end) = match repaired {
            Some((repair_start, repair_end)) => {
                (edit_start.min(repair_start), edit_end.max(repair_end))
            }
            None => (edit_start, edit_end),
        };
        let cols = end.saturating_sub(start);
        self.dirty.mark_span(self.cursor.row, start, cols);
        self.perf.dirty_cells += u64::from(cols);
    }

    pub(super) fn insert_blank_lines(&mut self, count: u16) {
        self.flush_dirty_run();
        if self.cursor.row < self.scroll_top || self.cursor.row > self.scroll_bottom {
            return;
        }
        let bottom = self.scroll_bottom;
        self.grid
            .insert_blank_rows_in_region(self.cursor.row, bottom, count, self.style);
        self.insert_hard_break_rows_in_region(self.cursor.row, bottom, count);
        let rows = bottom - self.cursor.row + 1;
        self.dirty.mark_region(DirtyRegion {
            row: self.cursor.row,
            col: 0,
            rows,
            cols: self.config.cols,
        });
        self.perf.dirty_cells += u64::from(rows) * u64::from(self.config.cols);
    }

    pub(super) fn delete_lines(&mut self, count: u16) {
        self.flush_dirty_run();
        if self.cursor.row < self.scroll_top || self.cursor.row > self.scroll_bottom {
            return;
        }
        let bottom = self.scroll_bottom;
        self.grid
            .delete_rows_in_region(self.cursor.row, bottom, count, self.style);
        self.delete_hard_break_rows_in_region(self.cursor.row, bottom, count);
        let rows = bottom - self.cursor.row + 1;
        self.dirty.mark_region(DirtyRegion {
            row: self.cursor.row,
            col: 0,
            rows,
            cols: self.config.cols,
        });
        self.perf.dirty_cells += u64::from(rows) * u64::from(self.config.cols);
    }

    pub(super) fn save_cursor(&mut self) {
        self.saved_cursor = Some(self.cursor);
    }

    pub(super) fn restore_cursor(&mut self) {
        if let Some(saved) = self.saved_cursor {
            self.cursor = saved;
        }
    }

    #[cold]
    #[inline(never)]
    pub(super) fn soft_reset(&mut self) {
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
                shape: CursorShape::Block,
                blinking: true,
            },
            style: Style::default(),
            g0_dec_special_graphics: false,
            g1_dec_special_graphics: false,
            active_charset: CharacterSet::G0,
        });
    }

    pub(super) fn reset_to_initial_state(&mut self) {
        self.flush_dirty_run();
        self.grid = Grid::new(self.config.cols, self.config.rows);
        self.hard_breaks = vec![false; usize::from(self.config.rows)];
        self.tab_stops = default_tab_stops(self.config.cols);
        self.scrollback = Scrollback::new(self.config.scrollback_limit);
        self.cursor = Cursor {
            row: 0,
            col: 0,
            visible: true,
            shape: CursorShape::Block,
            blinking: true,
        };
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

    pub(super) fn mark_print_span(&mut self, row: u16, col: u16, cols: u16) {
        if cols == 0 {
            return;
        }
        let col_end = col + cols;
        if self.dirty.contains_span(row, col, cols) {
            return;
        }
        match self.dirty_run {
            Some(run) if run.row == row && run.col_end >= col => {
                self.dirty_run = Some(DirtyRun {
                    row,
                    col_start: run.col_start.min(col),
                    col_end: run.col_end.max(col_end),
                });
            }
            Some(_) => {
                self.flush_dirty_run();
                self.dirty_run = Some(DirtyRun {
                    row,
                    col_start: col,
                    col_end,
                });
            }
            None => {
                self.dirty_run = Some(DirtyRun {
                    row,
                    col_start: col,
                    col_end,
                });
            }
        }
    }

    pub(super) fn flush_dirty_run(&mut self) {
        if let Some(run) = self.dirty_run.take() {
            self.dirty
                .mark_span(run.row, run.col_start, run.col_end - run.col_start);
        }
    }

    pub(super) fn reflow_visible_grid(&self, cols: u16, rows: u16) -> (Grid, Vec<bool>) {
        reflow::reflow_grid(&self.grid, &self.hard_breaks, cols, rows)
    }
}
