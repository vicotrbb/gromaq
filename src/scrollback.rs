//! Bounded scrollback storage.

use std::collections::VecDeque;

use unicode_width::UnicodeWidthChar;

use crate::cell::{CellSnapshot, Style};

mod line;
mod snapshot;

use line::{ScrollbackLine, cell_width, last_visible_cell, line_width, push_reflow_cell};
pub use snapshot::ScrollbackSnapshot;
use snapshot::logical_line_ids_for;

/// Bounded line scrollback.
#[derive(Debug, Clone)]
pub struct Scrollback {
    limit: usize,
    lines: VecDeque<ScrollbackLine>,
}

impl Scrollback {
    /// Create scrollback with a fixed line limit.
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            lines: VecDeque::with_capacity(limit.min(1024)),
        }
    }

    /// Update the retained line limit and evict oldest rows when the new limit is smaller.
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        while self.lines.len() > self.limit {
            self.lines.pop_front();
        }
    }

    /// Number of retained scrollback rows.
    pub(crate) fn len(&self) -> usize {
        self.lines.len()
    }

    /// Styled cells for a retained scrollback row.
    pub(crate) fn row_cells(&self, index: usize) -> Option<&[CellSnapshot]> {
        self.lines.get(index).map(|line| line.cells.as_slice())
    }

    /// Whether a retained scrollback row ended in a hard line break.
    pub(crate) fn hard_break_at(&self, index: usize) -> bool {
        self.lines.get(index).is_some_and(|line| line.hard_break)
    }

    /// Push one hard-break line, evicting the oldest line when capacity is reached.
    pub fn push(&mut self, line: String) {
        let cells = line
            .chars()
            .map(|ch| CellSnapshot {
                text: ch.to_string(),
                style: Style::default(),
                hyperlink_id: 0,
                is_wide_leading: UnicodeWidthChar::width(ch).unwrap_or(0).min(2) == 2,
                is_wide_trailing: false,
            })
            .collect();
        self.push_cells(cells);
    }

    /// Push one hard-break styled cell row, evicting the oldest line when capacity is reached.
    pub fn push_cells(&mut self, cells: Vec<CellSnapshot>) {
        self.push_cells_with_hard_break(cells, true);
    }

    /// Push one styled cell row, preserving whether it ended with a hard line break.
    pub(crate) fn push_cells_with_hard_break(
        &mut self,
        cells: Vec<CellSnapshot>,
        hard_break: bool,
    ) {
        if self.limit == 0 {
            return;
        }
        let Some(mut last_visible) = last_visible_cell(&cells) else {
            return;
        };
        if cells
            .get(last_visible + 1)
            .is_some_and(|cell| cell.is_wide_trailing)
        {
            last_visible += 1;
        }
        let cells = cells[..=last_visible].to_vec();
        if self.lines.len() == self.limit {
            self.lines.pop_front();
        }
        self.lines.push_back(ScrollbackLine { cells, hard_break });
    }

    /// Produce a stable snapshot.
    pub fn snapshot(&self) -> ScrollbackSnapshot {
        let hard_breaks: Vec<bool> = self.lines.iter().map(|line| line.hard_break).collect();
        let logical_line_ids = logical_line_ids_for(&hard_breaks);
        ScrollbackSnapshot {
            lines: self.lines.iter().map(ScrollbackLine::text).collect(),
            hard_breaks,
            logical_line_ids,
            hyperlinks: Vec::new(),
            underline_colors: Vec::new(),
            cells: self.lines.iter().map(|line| line.cells.clone()).collect(),
        }
    }

    /// Remove all retained scrollback rows.
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Rewrap existing plain-text scrollback rows from one terminal width to another.
    pub(crate) fn reflow(&mut self, old_cols: u16, new_cols: u16) {
        if old_cols == 0 || new_cols == 0 || old_cols == new_cols || self.lines.is_empty() {
            return;
        }
        let old_lines = std::mem::take(&mut self.lines);
        let mut logical_line = Vec::new();
        for line in old_lines {
            let is_soft_wrapped =
                !line.hard_break && line_width(&line.cells) >= usize::from(old_cols);
            logical_line.extend(line.cells);
            if !is_soft_wrapped {
                self.push_reflowed_cells(&logical_line, new_cols, line.hard_break);
                logical_line.clear();
            }
        }
        if !logical_line.is_empty() {
            self.push_reflowed_cells(&logical_line, new_cols, false);
        }
    }

    fn push_reflowed_cells(&mut self, line: &[CellSnapshot], cols: u16, hard_break: bool) {
        let cols = usize::from(cols);
        let mut current = Vec::new();
        let mut current_width = 0;
        for cell in line.iter().filter(|cell| !cell.is_wide_trailing) {
            let width = cell_width(cell);
            let width = if width == 2 && cols == 1 { 1 } else { width };
            if width == 0 {
                current.push(cell.clone());
                continue;
            }
            if current_width + width > cols && !current.is_empty() {
                self.push_cells_with_hard_break(std::mem::take(&mut current), false);
                current_width = 0;
            }
            push_reflow_cell(&mut current, cell, width);
            current_width += width;
        }
        if !current.is_empty() {
            self.push_cells_with_hard_break(current, hard_break);
        }
    }
}
