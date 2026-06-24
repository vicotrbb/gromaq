//! Visible-grid reflow helpers for terminal resize handling.

use crate::cell::{Cell, Style};
use crate::grid::Grid;

use super::width::visible_width;

/// Reflow a visible grid and matching hard-break metadata into a new size.
pub(super) fn reflow_grid(
    grid: &Grid,
    source_hard_breaks: &[bool],
    cols: u16,
    rows: u16,
) -> (Grid, Vec<bool>) {
    let lines = visible_logical_lines_for(grid, source_hard_breaks);
    let mut grid = Grid::new(cols, rows);
    let mut hard_breaks = vec![false; usize::from(rows)];
    let mut row = 0;
    let mut col = 0;

    for (line_index, line) in lines.iter().enumerate() {
        for unit in &line.cells {
            let width = if unit.width == 2 && cols > 1 { 2 } else { 1 };
            if col + width > cols {
                row += 1;
                col = 0;
            }
            if row >= rows {
                return (grid, hard_breaks);
            }
            *grid.cell_mut(row, col) = Cell {
                text: unit.text.clone(),
                style: unit.style,
                hyperlink_id: unit.hyperlink_id,
                is_wide_leading: width == 2,
                is_wide_trailing: false,
            };
            if width == 2 && col + 1 < cols {
                *grid.cell_mut(row, col + 1) = Cell {
                    text: String::new(),
                    style: unit.style,
                    hyperlink_id: unit.hyperlink_id,
                    is_wide_leading: false,
                    is_wide_trailing: true,
                };
            }
            col += width;
        }

        if line.hard_break && row < rows {
            hard_breaks[usize::from(row)] = true;
            if line_index + 1 < lines.len() {
                row += 1;
                col = 0;
            }
        } else if col >= cols {
            row += 1;
            col = 0;
        }
    }

    (grid, hard_breaks)
}

fn visible_logical_lines_for(grid: &Grid, source_hard_breaks: &[bool]) -> Vec<LogicalLine> {
    let mut lines = Vec::new();
    let mut current = Vec::new();

    for row in 0..grid.rows() {
        let cells = visible_row_units_for(grid, row);
        let is_hard_break = source_hard_breaks
            .get(usize::from(row))
            .copied()
            .unwrap_or(false);
        let is_full_soft_row = !is_hard_break
            && cells
                .iter()
                .map(|cell| usize::from(cell.width))
                .sum::<usize>()
                >= usize::from(grid.cols());

        if !cells.is_empty() {
            current.extend(cells);
        }

        if is_hard_break {
            lines.push(LogicalLine {
                cells: std::mem::take(&mut current),
                hard_break: true,
            });
        } else if !is_full_soft_row && !current.is_empty() {
            lines.push(LogicalLine {
                cells: std::mem::take(&mut current),
                hard_break: false,
            });
        }
    }

    if !current.is_empty() {
        lines.push(LogicalLine {
            cells: current,
            hard_break: false,
        });
    }

    lines
}

fn visible_row_units_for(grid: &Grid, row: u16) -> Vec<ReflowCell> {
    let Some(last_col) = last_visible_col_for(grid, row) else {
        return Vec::new();
    };

    let mut units = Vec::new();
    for col in 0..=last_col {
        let cell = grid.cell(row, col);
        if cell.is_wide_trailing {
            continue;
        }
        if cell.text.is_empty() {
            units.push(ReflowCell {
                text: " ".to_owned(),
                style: cell.style,
                hyperlink_id: cell.hyperlink_id,
                width: 1,
            });
            continue;
        }
        let width = if cell.is_wide_leading || visible_width(&cell.text) >= 2 {
            2
        } else {
            1
        };
        units.push(ReflowCell {
            text: cell.text.clone(),
            style: cell.style,
            hyperlink_id: cell.hyperlink_id,
            width,
        });
    }
    units
}

fn last_visible_col_for(grid: &Grid, row: u16) -> Option<u16> {
    (0..grid.cols()).rev().find(|col| {
        let cell = grid.cell(row, *col);
        !cell.text.is_empty() && !cell.is_wide_trailing
    })
}

#[derive(Debug)]
struct LogicalLine {
    cells: Vec<ReflowCell>,
    hard_break: bool,
}

#[derive(Debug, Clone)]
struct ReflowCell {
    text: String,
    style: Style,
    hyperlink_id: u16,
    width: u16,
}
