//! Bounded scrollback storage.

use std::collections::VecDeque;

use unicode_width::UnicodeWidthChar;

use crate::cell::{CellSnapshot, Color, Style};

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

#[derive(Debug, Clone)]
struct ScrollbackLine {
    cells: Vec<CellSnapshot>,
    hard_break: bool,
}

impl ScrollbackLine {
    fn text(&self) -> String {
        line_text(&self.cells)
    }
}

fn push_reflow_cell(target: &mut Vec<CellSnapshot>, cell: &CellSnapshot, width: usize) {
    let mut leading = cell.clone();
    leading.is_wide_leading = width == 2;
    leading.is_wide_trailing = false;
    target.push(leading);
    if width == 2 {
        target.push(CellSnapshot {
            text: String::new(),
            style: cell.style,
            hyperlink_id: cell.hyperlink_id,
            is_wide_leading: false,
            is_wide_trailing: true,
        });
    }
}

fn line_width(line: &[CellSnapshot]) -> usize {
    line.iter()
        .filter(|cell| !cell.is_wide_trailing)
        .map(cell_width)
        .sum()
}

fn line_text(cells: &[CellSnapshot]) -> String {
    let mut output = String::new();
    for cell in cells {
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

fn cell_width(cell: &CellSnapshot) -> usize {
    if cell.is_wide_leading {
        2
    } else if cell.text.is_empty() {
        1
    } else {
        line_text_width(&cell.text).clamp(1, 2)
    }
}

fn line_text_width(text: &str) -> usize {
    text.chars()
        .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0).min(2))
        .sum()
}

fn last_visible_cell(cells: &[CellSnapshot]) -> Option<usize> {
    cells
        .iter()
        .rposition(|cell| !cell.text.is_empty() && !cell.is_wide_trailing)
}

fn logical_line_ids_for(hard_breaks: &[bool]) -> Vec<usize> {
    let mut current_id = 0;
    let mut ids = Vec::with_capacity(hard_breaks.len());
    for hard_break in hard_breaks {
        ids.push(current_id);
        if *hard_break {
            current_id += 1;
        }
    }
    ids
}

/// Immutable scrollback snapshot used by tests and debug tooling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollbackSnapshot {
    /// Scrollback lines from oldest to newest.
    pub lines: Vec<String>,
    /// Whether each scrollback row ended a hard line break instead of a soft wrap.
    pub hard_breaks: Vec<bool>,
    /// Stable logical-line group for each retained physical scrollback row.
    pub logical_line_ids: Vec<usize>,
    /// OSC 8 hyperlink URI table indexed by non-zero cell hyperlink identifiers.
    pub hyperlinks: Vec<String>,
    /// Underline color table indexed by non-zero style underline color identifiers.
    pub underline_colors: Vec<Color>,
    /// Styled scrollback cells from oldest to newest row.
    pub cells: Vec<Vec<CellSnapshot>>,
}
