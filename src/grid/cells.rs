//! Cell-level editing operations for the fixed terminal grid.

use super::Grid;
use crate::cell::{Cell, Style};

impl Grid {
    /// Clear one cell.
    pub fn clear_cell(&mut self, row: u16, col: u16, style: Style) {
        *self.cell_mut(row, col) = Cell::blank(style);
    }

    /// Clear one row.
    pub fn clear_row(&mut self, row: u16, style: Style) {
        for col in 0..self.cols {
            self.clear_cell(row, col, style);
        }
    }

    /// Insert blank cells at `col`, shifting the row right and dropping overflow.
    pub fn insert_blank_cells(&mut self, row: u16, col: u16, count: u16, style: Style) {
        if col >= self.cols || count == 0 {
            return;
        }
        let count = count.min(self.cols - col);
        for target_col in (col + count..self.cols).rev() {
            let source = self.cell(row, target_col - count).clone();
            *self.cell_mut(row, target_col) = source;
        }
        for blank_col in col..col + count {
            self.clear_cell(row, blank_col, style);
        }
    }

    /// Delete cells at `col`, shifting the row left and blanking the right edge.
    pub fn delete_cells(&mut self, row: u16, col: u16, count: u16, style: Style) {
        if col >= self.cols || count == 0 {
            return;
        }
        let count = count.min(self.cols - col);
        for target_col in col..self.cols - count {
            let source = self.cell(row, target_col + count).clone();
            *self.cell_mut(row, target_col) = source;
        }
        for blank_col in self.cols - count..self.cols {
            self.clear_cell(row, blank_col, style);
        }
    }

    /// Clear split wide-cell fragments left by cell-wise row mutations.
    ///
    /// Returns the repaired column range as `(start, end_exclusive)` when cells changed.
    pub fn repair_wide_cells_in_row(&mut self, row: u16, style: Style) -> Option<(u16, u16)> {
        let mut col = 0;
        let mut repaired_start = self.cols;
        let mut repaired_end = 0;
        while col < self.cols {
            let cell = self.cell(row, col);
            if cell.is_wide_trailing {
                self.clear_cell(row, col, style);
                repaired_start = repaired_start.min(col);
                repaired_end = repaired_end.max(col + 1);
                col += 1;
                continue;
            }

            if cell.is_wide_leading {
                let has_valid_trailing = col + 1 < self.cols && {
                    let trailing = self.cell(row, col + 1);
                    trailing.is_wide_trailing && trailing.text.is_empty()
                };
                if has_valid_trailing {
                    col += 2;
                } else {
                    self.clear_cell(row, col, style);
                    repaired_start = repaired_start.min(col);
                    repaired_end = repaired_end.max(col + 1);
                    col += 1;
                }
                continue;
            }

            col += 1;
        }
        (repaired_start < repaired_end).then_some((repaired_start, repaired_end))
    }
}
