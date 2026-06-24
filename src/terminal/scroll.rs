//! Line-feed, index, and scroll-region behavior.

use crate::dirty::DirtyRegion;

use super::Terminal;

impl Terminal {
    pub(super) fn line_feed(&mut self) {
        self.flush_dirty_run();
        if self.cursor.row == self.scroll_bottom {
            self.scroll_region_up(1);
        } else if self.cursor.row + 1 < self.config.rows {
            self.cursor.row += 1;
        }
    }

    pub(super) fn index(&mut self) {
        self.wrap_pending = false;
        self.line_feed();
    }

    pub(super) fn next_line(&mut self) {
        self.wrap_pending = false;
        self.line_feed();
        self.carriage_return();
    }

    pub(super) fn scroll_region_up(&mut self, count: u16) {
        let region_rows = self.scroll_bottom - self.scroll_top + 1;
        if self.scroll_top == 0 && self.scroll_bottom == self.config.rows - 1 && count == 1 {
            let removed = self.trimmed_visible_row_snapshot(0);
            let removed_hard_break = self.hard_breaks.first().copied().unwrap_or(false);
            self.grid.scroll_up_one(self.style);
            if self.saved_primary.is_none() {
                self.scrollback
                    .push_cells_with_hard_break(removed, removed_hard_break);
            }
            if !self.hard_breaks.is_empty() {
                self.hard_breaks.rotate_left(1);
                if let Some(last) = self.hard_breaks.last_mut() {
                    *last = false;
                }
            }
            self.dirty.mark_viewport(self.config.rows, self.config.cols);
            self.perf.dirty_cells += u64::from(self.config.rows) * u64::from(self.config.cols);
            self.perf.scrolls += 1;
        } else {
            let count = count.min(region_rows);
            self.grid
                .scroll_region_up(self.scroll_top, self.scroll_bottom, count, self.style);
            self.delete_hard_break_rows_in_region(self.scroll_top, self.scroll_bottom, count);
            self.dirty.mark_region(DirtyRegion {
                row: self.scroll_top,
                col: 0,
                rows: region_rows,
                cols: self.config.cols,
            });
            self.perf.dirty_cells += u64::from(region_rows) * u64::from(self.config.cols);
            self.perf.scrolls += u64::from(count);
        }
    }

    pub(super) fn reverse_index(&mut self) {
        self.flush_dirty_run();
        self.wrap_pending = false;
        if self.cursor.row == self.scroll_top {
            self.scroll_region_down(1);
        } else {
            self.cursor.row = self.cursor.row.saturating_sub(1);
        }
    }

    pub(super) fn scroll_region_down(&mut self, count: u16) {
        let region_rows = self.scroll_bottom - self.scroll_top + 1;
        let count = count.min(region_rows);
        self.grid
            .scroll_region_down(self.scroll_top, self.scroll_bottom, count, self.style);
        self.insert_hard_break_rows_in_region(self.scroll_top, self.scroll_bottom, count);
        self.dirty.mark_region(DirtyRegion {
            row: self.scroll_top,
            col: 0,
            rows: region_rows,
            cols: self.config.cols,
        });
        self.perf.dirty_cells += u64::from(region_rows) * u64::from(self.config.cols);
        self.perf.scrolls += u64::from(count);
    }

    pub(super) fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        let top = top.saturating_sub(1);
        let bottom = bottom.saturating_sub(1).min(self.config.rows - 1);
        if top >= bottom {
            return;
        }
        self.flush_dirty_run();
        self.scroll_top = top;
        self.scroll_bottom = bottom;
        self.cursor.row = if self.origin_mode { self.scroll_top } else { 0 };
        self.cursor.col = 0;
        self.wrap_pending = false;
    }

    pub(super) fn scroll_viewport_up(&mut self, count: u16) {
        self.flush_dirty_run();
        self.wrap_pending = false;
        self.scroll_region_up(count);
    }

    pub(super) fn scroll_viewport_down(&mut self, count: u16) {
        self.flush_dirty_run();
        self.wrap_pending = false;
        self.scroll_region_down(count);
    }

    pub(super) fn delete_hard_break_rows_in_region(&mut self, top: u16, bottom: u16, count: u16) {
        if top >= self.config.rows || bottom >= self.config.rows || top >= bottom || count == 0 {
            return;
        }
        let height = bottom - top + 1;
        let count = count.min(height);
        for target_row in top..=bottom - count {
            self.hard_breaks[usize::from(target_row)] =
                self.hard_breaks[usize::from(target_row + count)];
        }
        for blank_row in bottom - count + 1..=bottom {
            self.hard_breaks[usize::from(blank_row)] = false;
        }
    }

    pub(super) fn insert_hard_break_rows_in_region(&mut self, top: u16, bottom: u16, count: u16) {
        if top >= self.config.rows || bottom >= self.config.rows || top >= bottom || count == 0 {
            return;
        }
        let height = bottom - top + 1;
        let count = count.min(height);
        for target_row in (top + count..=bottom).rev() {
            self.hard_breaks[usize::from(target_row)] =
                self.hard_breaks[usize::from(target_row - count)];
        }
        for blank_row in top..top + count {
            self.hard_breaks[usize::from(blank_row)] = false;
        }
    }
}
