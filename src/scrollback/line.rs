use unicode_width::UnicodeWidthChar;

use crate::cell::CellSnapshot;

#[derive(Debug, Clone)]
pub(super) struct ScrollbackLine {
    pub(super) cells: Vec<CellSnapshot>,
    pub(super) hard_break: bool,
}

impl ScrollbackLine {
    pub(super) fn text(&self) -> String {
        line_text(&self.cells)
    }
}

pub(super) fn push_reflow_cell(target: &mut Vec<CellSnapshot>, cell: &CellSnapshot, width: usize) {
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

pub(super) fn line_width(line: &[CellSnapshot]) -> usize {
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

pub(super) fn cell_width(cell: &CellSnapshot) -> usize {
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

pub(super) fn last_visible_cell(cells: &[CellSnapshot]) -> Option<usize> {
    cells
        .iter()
        .rposition(|cell| !cell.text.is_empty() && !cell.is_wide_trailing)
}
