//! Fixed-size terminal grid storage and snapshots.

use crate::cell::{Cell, CellSnapshot, Color, Style};

/// Mutable terminal grid.
#[derive(Debug, Clone)]
pub struct Grid {
    cols: u16,
    rows: u16,
    cells: Vec<Cell>,
}

impl Grid {
    /// Create a blank grid with `cols * rows` cells.
    pub fn new(cols: u16, rows: u16) -> Self {
        let len = usize::from(cols) * usize::from(rows);
        Self {
            cols,
            rows,
            cells: vec![Cell::default(); len],
        }
    }

    /// Number of columns.
    pub fn cols(&self) -> u16 {
        self.cols
    }

    /// Number of rows.
    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// Immutable access to a cell.
    pub fn cell(&self, row: u16, col: u16) -> &Cell {
        &self.cells[self.index(row, col)]
    }

    /// Mutable access to a cell.
    pub fn cell_mut(&mut self, row: u16, col: u16) -> &mut Cell {
        let index = self.index(row, col);
        &mut self.cells[index]
    }

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

    /// Convert a row to visible text, preserving leading spaces and trimming trailing blanks.
    pub fn line_text(&self, row: u16) -> String {
        let mut output = String::new();
        for col in 0..self.cols {
            let cell = self.cell(row, col);
            if cell.is_wide_trailing {
                continue;
            }
            if cell.text.is_empty() {
                output.push(' ');
            } else {
                output.push_str(&cell.text);
            }
        }
        output.trim_end().to_owned()
    }

    /// Return one row as cell snapshots.
    pub fn row_snapshot(&self, row: u16) -> Vec<CellSnapshot> {
        (0..self.cols)
            .map(|col| CellSnapshot::from(self.cell(row, col)))
            .collect()
    }

    /// Return one row as cell snapshots, excluding trailing blank cells.
    pub fn trimmed_row_snapshot(&self, row: u16) -> Vec<CellSnapshot> {
        let Some(last_col) = (0..self.cols).rev().find(|col| {
            let cell = self.cell(row, *col);
            !cell.text.is_empty() && !cell.is_wide_trailing
        }) else {
            return Vec::new();
        };
        (0..=last_col)
            .map(|col| CellSnapshot::from(self.cell(row, col)))
            .collect()
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

    /// Produce a stable grid snapshot.
    pub fn snapshot(&self) -> GridSnapshot {
        GridSnapshot {
            cols: self.cols,
            rows: self.rows,
            hyperlinks: Vec::new(),
            underline_colors: Vec::new(),
            cells: self.cells.iter().map(CellSnapshot::from).collect(),
        }
    }

    fn index(&self, row: u16, col: u16) -> usize {
        debug_assert!(row < self.rows);
        debug_assert!(col < self.cols);
        usize::from(row) * usize::from(self.cols) + usize::from(col)
    }
}

/// Immutable grid snapshot used by tests and debug tooling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridSnapshot {
    /// Number of columns.
    pub cols: u16,
    /// Number of rows.
    pub rows: u16,
    /// OSC 8 hyperlink URI table indexed by non-zero cell hyperlink identifiers.
    pub hyperlinks: Vec<String>,
    /// Underline color table indexed by non-zero style underline color identifiers.
    pub underline_colors: Vec<Color>,
    /// Row-major cell snapshots.
    pub cells: Vec<CellSnapshot>,
}

impl GridSnapshot {
    /// Return a cell snapshot by row and column.
    ///
    /// Panics when `row` or `col` is outside the snapshot dimensions.
    pub fn cell(&self, row: u16, col: u16) -> &CellSnapshot {
        let index = usize::from(row) * usize::from(self.cols) + usize::from(col);
        &self.cells[index]
    }

    /// Return the OSC 8 hyperlink URI for a cell, when present.
    pub fn cell_hyperlink(&self, row: u16, col: u16) -> Option<&str> {
        let hyperlink_id = self.cell(row, col).hyperlink_id;
        if hyperlink_id == 0 {
            return None;
        }
        self.hyperlinks
            .get(usize::from(hyperlink_id - 1))
            .map(String::as_str)
    }

    /// Return the resolved underline color for a cell.
    pub fn cell_underline_color(&self, row: u16, col: u16) -> Color {
        let underline_color_id = self.cell(row, col).style.underline_color_id;
        if underline_color_id == 0 {
            return Color::Default;
        }
        self.underline_colors
            .get(usize::from(underline_color_id - 1))
            .copied()
            .unwrap_or(Color::Default)
    }

    /// Convert a row to visible text, preserving leading spaces and trimming trailing blanks.
    pub fn line_text(&self, row: u16) -> String {
        let mut output = String::new();
        for col in 0..self.cols {
            let cell = self.cell(row, col);
            if cell.is_wide_trailing {
                continue;
            }
            if cell.text.is_empty() {
                output.push(' ');
            } else {
                output.push_str(&cell.text);
            }
        }
        output.trim_end().to_owned()
    }
}
