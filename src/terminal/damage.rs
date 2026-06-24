//! Dirty-run batching and visible-grid damage helpers.

use crate::grid::Grid;

use super::Terminal;
use super::reflow;
use super::state::DirtyRun;

impl Terminal {
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
