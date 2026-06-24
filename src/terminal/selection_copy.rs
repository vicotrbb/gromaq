//! Visible-grid selection and plain-text copy extraction for terminal state.

use crate::clipboard::HostClipboard;
use crate::grid::GridSnapshot;
use crate::selection::{SelectionPoint, SelectionRange};

use super::Terminal;

impl Terminal {
    /// Set a visible-grid selection.
    pub fn set_selection(&mut self, selection: SelectionRange) {
        self.selection = Some(selection);
    }

    /// Clear the active selection.
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Copy the active selection as plain text.
    pub fn copy_selection(&self) -> Option<String> {
        let selection = self.selection?;
        Some(self.copy_range(selection))
    }

    /// Copy the active selection into a host clipboard adapter.
    pub fn copy_selection_to_clipboard(
        &self,
        clipboard: &mut impl HostClipboard,
    ) -> Option<String> {
        let text = self.copy_selection()?;
        clipboard.write_text(&text);
        Some(text)
    }

    fn copy_range(&self, selection: SelectionRange) -> String {
        let selection = self.clamp_selection_to_viewport(selection);
        let grid = self.dump_grid();
        let hard_breaks = self.displayed_hard_breaks();
        let mut output = String::new();
        for row in selection.start.row..=selection.end.row {
            let start_col = if row == selection.start.row {
                selection.start.col
            } else {
                0
            };
            let end_col = if row == selection.end.row {
                selection.end.col
            } else {
                self.config.cols - 1
            };
            output.push_str(&self.copy_row_range(&grid, row, start_col, end_col));
            if row < selection.end.row
                && self.copy_boundary_needs_newline(&hard_breaks, row, end_col)
            {
                output.push('\n');
            }
        }
        output
    }

    fn clamp_selection_to_viewport(&self, selection: SelectionRange) -> SelectionRange {
        let start = self.clamp_selection_point(selection.start);
        let end = self.clamp_selection_point(selection.end);
        if start <= end {
            SelectionRange { start, end }
        } else {
            SelectionRange {
                start: end,
                end: start,
            }
        }
    }

    fn clamp_selection_point(&self, point: SelectionPoint) -> SelectionPoint {
        SelectionPoint {
            row: point.row.min(self.config.rows - 1),
            col: point.col.min(self.config.cols - 1),
        }
    }

    fn copy_row_range(
        &self,
        grid: &GridSnapshot,
        row: u16,
        start_col: u16,
        end_col: u16,
    ) -> String {
        let start_col = self.copy_start_col(grid, row, start_col);
        let Some(end_col) = self
            .last_visible_col_in_row(grid, row)
            .map(|last_col| end_col.min(last_col))
        else {
            return String::new();
        };

        if end_col < start_col {
            return String::new();
        }

        let mut output = String::new();
        for col in start_col..=end_col {
            let cell = grid.cell(row, col);
            if cell.is_wide_trailing {
                continue;
            }
            if cell.text.is_empty() {
                output.push(' ');
            } else {
                output.push_str(&cell.text);
            }
        }
        output
    }

    fn copy_start_col(&self, grid: &GridSnapshot, row: u16, start_col: u16) -> u16 {
        if start_col > 0 && grid.cell(row, start_col).is_wide_trailing {
            start_col - 1
        } else {
            start_col
        }
    }

    fn last_visible_col_in_row(&self, grid: &GridSnapshot, row: u16) -> Option<u16> {
        (0..self.config.cols).rev().find(|col| {
            let cell = grid.cell(row, *col);
            !cell.text.is_empty() && !cell.is_wide_trailing
        })
    }

    fn copy_boundary_needs_newline(&self, hard_breaks: &[bool], row: u16, end_col: u16) -> bool {
        hard_breaks.get(usize::from(row)).copied().unwrap_or(false)
            || end_col < self.config.cols - 1
    }

    fn displayed_hard_breaks(&self) -> Vec<bool> {
        if self.scrollback_view_offset == 0 {
            return self.hard_breaks.clone();
        }

        let scrollback = self.scrollback.snapshot();
        let history_rows = scrollback.hard_breaks.len();
        let visible_rows = usize::from(self.config.rows);
        let offset = self.scrollback_view_offset.min(history_rows);
        let start = (history_rows + visible_rows).saturating_sub(visible_rows + offset);

        (0..visible_rows)
            .map(|row| {
                let source_row = start + row;
                if source_row < history_rows {
                    scrollback
                        .hard_breaks
                        .get(source_row)
                        .copied()
                        .unwrap_or(false)
                } else {
                    self.hard_breaks
                        .get(source_row - history_rows)
                        .copied()
                        .unwrap_or(false)
                }
            })
            .collect()
    }
}
