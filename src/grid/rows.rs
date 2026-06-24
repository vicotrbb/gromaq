//! Row, scroll-region, and resize mutations for the fixed terminal grid.

use super::Grid;
use crate::cell::Style;

impl Grid {
    /// Insert blank rows at `row`, shifting rows downward and dropping bottom rows.
    pub fn insert_blank_rows(&mut self, row: u16, count: u16, style: Style) {
        if row >= self.rows || count == 0 {
            return;
        }
        let count = count.min(self.rows - row);
        for target_row in (row + count..self.rows).rev() {
            for col in 0..self.cols {
                let source = self.cell(target_row - count, col).clone();
                *self.cell_mut(target_row, col) = source;
            }
        }
        for blank_row in row..row + count {
            self.clear_row(blank_row, style);
        }
    }

    /// Delete rows at `row`, shifting rows upward and blanking bottom rows.
    pub fn delete_rows(&mut self, row: u16, count: u16, style: Style) {
        if row >= self.rows || count == 0 {
            return;
        }
        let count = count.min(self.rows - row);
        for target_row in row..self.rows - count {
            for col in 0..self.cols {
                let source = self.cell(target_row + count, col).clone();
                *self.cell_mut(target_row, col) = source;
            }
        }
        for blank_row in self.rows - count..self.rows {
            self.clear_row(blank_row, style);
        }
    }

    /// Insert blank rows inside an inclusive row region, dropping rows at the region bottom.
    pub fn insert_blank_rows_in_region(&mut self, row: u16, bottom: u16, count: u16, style: Style) {
        if row >= self.rows || bottom >= self.rows || row > bottom || count == 0 {
            return;
        }
        let height = bottom - row + 1;
        let count = count.min(height);
        for target_row in (row + count..=bottom).rev() {
            for col in 0..self.cols {
                let source = self.cell(target_row - count, col).clone();
                *self.cell_mut(target_row, col) = source;
            }
        }
        for blank_row in row..row + count {
            self.clear_row(blank_row, style);
        }
    }

    /// Delete rows inside an inclusive row region, blanking rows at the region bottom.
    pub fn delete_rows_in_region(&mut self, row: u16, bottom: u16, count: u16, style: Style) {
        if row >= self.rows || bottom >= self.rows || row > bottom || count == 0 {
            return;
        }
        let height = bottom - row + 1;
        let count = count.min(height);
        for target_row in row..=bottom - count {
            for col in 0..self.cols {
                let source = self.cell(target_row + count, col).clone();
                *self.cell_mut(target_row, col) = source;
            }
        }
        for blank_row in bottom - count + 1..=bottom {
            self.clear_row(blank_row, style);
        }
    }

    /// Scroll an inclusive row region upward, blanking rows at the bottom edge.
    pub fn scroll_region_up(&mut self, top: u16, bottom: u16, count: u16, style: Style) {
        if top >= self.rows || bottom >= self.rows || top >= bottom || count == 0 {
            return;
        }
        let height = bottom - top + 1;
        let count = count.min(height);
        for target_row in top..=bottom - count {
            for col in 0..self.cols {
                let source = self.cell(target_row + count, col).clone();
                *self.cell_mut(target_row, col) = source;
            }
        }
        for blank_row in bottom - count + 1..=bottom {
            self.clear_row(blank_row, style);
        }
    }

    /// Scroll an inclusive row region downward, blanking rows at the top edge.
    pub fn scroll_region_down(&mut self, top: u16, bottom: u16, count: u16, style: Style) {
        if top >= self.rows || bottom >= self.rows || top >= bottom || count == 0 {
            return;
        }
        let height = bottom - top + 1;
        let count = count.min(height);
        for target_row in (top + count..=bottom).rev() {
            for col in 0..self.cols {
                let source = self.cell(target_row - count, col).clone();
                *self.cell_mut(target_row, col) = source;
            }
        }
        for blank_row in top..top + count {
            self.clear_row(blank_row, style);
        }
    }

    /// Scroll one line up.
    pub fn scroll_up_one(&mut self, style: Style) {
        if self.rows > 1 {
            self.cells.rotate_left(usize::from(self.cols));
        }
        self.clear_row(self.rows - 1, style);
    }

    /// Resize while preserving top-left visible content.
    pub fn resize_preserve(&mut self, cols: u16, rows: u16) {
        let old = self.clone();
        let mut resized = Self::new(cols, rows);
        let copy_rows = old.rows.min(rows);
        let copy_cols = old.cols.min(cols);
        for row in 0..copy_rows {
            for col in 0..copy_cols {
                *resized.cell_mut(row, col) = old.cell(row, col).clone();
            }
        }
        *self = resized;
    }
}
