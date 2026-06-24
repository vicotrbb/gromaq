use crate::cell::{Cell, CellSnapshot};
use crate::grid::GridSnapshot;
use crate::scrollback::ScrollbackSnapshot;

use super::Terminal;
use super::snapshot::push_snapshot_row;
use super::types::CursorSnapshot;

impl Terminal {
    /// Return a grid snapshot.
    pub fn dump_grid(&self) -> GridSnapshot {
        if self.scrollback_view_offset != 0 {
            return self.dump_scrollback_view_grid();
        }
        GridSnapshot {
            cols: self.config.cols,
            rows: self.config.rows,
            hyperlinks: self.hyperlinks.clone(),
            underline_colors: self.underline_colors.clone(),
            cells: (0..self.config.rows)
                .flat_map(|row| {
                    (0..self.config.cols)
                        .map(move |col| self.snapshot_cell(self.grid.cell(row, col)))
                })
                .collect(),
        }
    }

    fn dump_scrollback_view_grid(&self) -> GridSnapshot {
        let scrollback = self.scrollback.snapshot();
        let history_rows = scrollback.cells.len();
        let visible_rows = usize::from(self.config.rows);
        let offset = self.scrollback_view_offset.min(history_rows);
        let start = (history_rows + visible_rows).saturating_sub(visible_rows + offset);
        let mut cells = Vec::with_capacity(visible_rows * usize::from(self.config.cols));

        for row in 0..visible_rows {
            let source_row = start + row;
            if source_row < history_rows {
                push_snapshot_row(
                    &mut cells,
                    scrollback.cells.get(source_row).map(Vec::as_slice),
                    self.config.cols,
                );
            } else {
                let grid_row = (source_row - history_rows) as u16;
                let live_row = self.trimmed_visible_row_snapshot(grid_row);
                push_snapshot_row(&mut cells, Some(&live_row), self.config.cols);
            }
        }

        GridSnapshot {
            cols: self.config.cols,
            rows: self.config.rows,
            hyperlinks: self.hyperlinks.clone(),
            underline_colors: self.underline_colors.clone(),
            cells,
        }
    }

    fn snapshot_cell(&self, cell: &Cell) -> CellSnapshot {
        CellSnapshot {
            text: cell.text.clone(),
            style: cell.style,
            hyperlink_id: cell.hyperlink_id,
            is_wide_leading: cell.is_wide_leading,
            is_wide_trailing: cell.is_wide_trailing,
        }
    }

    pub(super) fn trimmed_visible_row_snapshot(&self, row: u16) -> Vec<CellSnapshot> {
        let Some(last_col) = (0..self.config.cols).rev().find(|col| {
            let cell = self.grid.cell(row, *col);
            !cell.text.is_empty() && !cell.is_wide_trailing
        }) else {
            return Vec::new();
        };
        let last_col = if last_col + 1 < self.config.cols
            && self.grid.cell(row, last_col + 1).is_wide_trailing
        {
            last_col + 1
        } else {
            last_col
        };
        (0..=last_col)
            .map(|col| self.snapshot_cell(self.grid.cell(row, col)))
            .collect()
    }

    /// Return a scrollback snapshot.
    pub fn dump_scrollback(&self) -> ScrollbackSnapshot {
        let mut snapshot = self.scrollback.snapshot();
        snapshot.hyperlinks.clone_from(&self.hyperlinks);
        snapshot.underline_colors.clone_from(&self.underline_colors);
        snapshot
    }

    /// Return a cursor snapshot.
    pub fn dump_cursor(&self) -> CursorSnapshot {
        CursorSnapshot {
            row: self.cursor.row,
            col: self.cursor.col,
            visible: self.cursor.visible && self.scrollback_view_offset == 0,
            shape: self.cursor.shape,
            blinking: self.cursor.blinking,
        }
    }

    /// Whether the terminal is currently displaying the alternate screen buffer.
    pub fn is_alternate_screen_active(&self) -> bool {
        self.saved_primary.is_some()
    }

    /// Scroll the displayed viewport upward into retained scrollback rows.
    pub fn scroll_display_up(&mut self, rows: u16) -> bool {
        if rows == 0 || self.saved_primary.is_some() {
            return false;
        }
        let max_offset = self.scrollback.len();
        let next = self
            .scrollback_view_offset
            .saturating_add(usize::from(rows))
            .min(max_offset);
        self.set_scrollback_view_offset(next)
    }

    /// Scroll the displayed viewport downward toward the live grid.
    pub fn scroll_display_down(&mut self, rows: u16) -> bool {
        if rows == 0 {
            return false;
        }
        let next = self
            .scrollback_view_offset
            .saturating_sub(usize::from(rows));
        self.set_scrollback_view_offset(next)
    }

    pub(super) fn scroll_display_to_bottom(&mut self) -> bool {
        self.set_scrollback_view_offset(0)
    }

    fn set_scrollback_view_offset(&mut self, offset: usize) -> bool {
        let offset = offset.min(self.scrollback.len());
        if offset == self.scrollback_view_offset {
            return false;
        }
        let moved_rows = self.scrollback_view_offset.abs_diff(offset);
        self.scrollback_view_offset = offset;
        self.selection = None;
        self.flush_dirty_run();
        self.dirty.mark_viewport(self.config.rows, self.config.cols);
        self.perf.scrolls += u64::try_from(moved_rows).unwrap_or(u64::MAX);
        self.perf.dirty_cells += u64::from(self.config.rows) * u64::from(self.config.cols);
        true
    }
}
