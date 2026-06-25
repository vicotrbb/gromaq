use crate::selection::SelectionRange;

use super::PlannedBackground;

pub(super) fn is_selected(selection: Option<SelectionRange>, row: u16, col: u16) -> bool {
    let Some(selection) = selection else {
        return false;
    };
    row >= selection.start.row
        && row <= selection.end.row
        && col >= selection_start_col(selection, row)
        && col <= selection_end_col(selection, row)
}

fn selection_start_col(selection: SelectionRange, row: u16) -> u16 {
    if row == selection.start.row {
        selection.start.col
    } else {
        0
    }
}

fn selection_end_col(selection: SelectionRange, row: u16) -> u16 {
    if row == selection.end.row {
        selection.end.col
    } else {
        u16::MAX
    }
}

pub(super) fn append_background_fill(
    backgrounds: &mut Vec<PlannedBackground>,
    row: u16,
    col: u16,
    color_rgba8: [u8; 4],
) {
    if let Some(last) = backgrounds.last_mut()
        && last.row == row
        && last.col.saturating_add(last.cols) == col
        && last.color_rgba8 == color_rgba8
    {
        last.cols = last.cols.saturating_add(1);
        return;
    }
    backgrounds.push(PlannedBackground {
        row,
        col,
        cols: 1,
        color_rgba8,
    });
}
