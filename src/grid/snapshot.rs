use crate::cell::{CellSnapshot, Color};
use crate::selection::SelectionRange;

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
    /// Active visible-grid selection, when present.
    pub selection: Option<SelectionRange>,
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
