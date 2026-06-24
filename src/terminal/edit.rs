//! VT editing, cursor movement, scrolling, and dirty-region helpers.

use crate::cell::Cell;
use crate::dirty::DirtyRegion;

use super::Terminal;

mod reset;

impl Terminal {
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
}
