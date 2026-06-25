//! Fixed-size terminal grid storage and snapshots.

mod cells;
mod rows;
mod snapshot;

pub use snapshot::GridSnapshot;

use crate::cell::{Cell, CellSnapshot};

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

    /// Produce a stable grid snapshot.
    pub fn snapshot(&self) -> GridSnapshot {
        GridSnapshot {
            cols: self.cols,
            rows: self.rows,
            hyperlinks: Vec::new(),
            underline_colors: Vec::new(),
            selection: None,
            cells: self.cells.iter().map(CellSnapshot::from).collect(),
        }
    }

    fn index(&self, row: u16, col: u16) -> usize {
        debug_assert!(row < self.rows);
        debug_assert!(col < self.cols);
        usize::from(row) * usize::from(self.cols) + usize::from(col)
    }
}
