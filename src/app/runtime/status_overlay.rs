use crate::{CellSnapshot, Color, CursorSnapshot, DirtyRegion, GridSnapshot, Style};

const STATUS_RIGHT_MARGIN_COLS: u16 = 1;

pub(super) fn apply_status_overlay(
    grid: &mut GridSnapshot,
    cursor: CursorSnapshot,
    status: &str,
) -> Option<DirtyRegion> {
    apply_status_overlay_with_fallback(grid, cursor, status, false)
}

pub(super) fn apply_status_overlay_nearby(
    grid: &mut GridSnapshot,
    cursor: CursorSnapshot,
    status: &str,
) -> Option<DirtyRegion> {
    apply_status_overlay_with_fallback(grid, cursor, status, true)
}

fn apply_status_overlay_with_fallback(
    grid: &mut GridSnapshot,
    cursor: CursorSnapshot,
    status: &str,
    fallback: bool,
) -> Option<DirtyRegion> {
    let status = status.trim();
    if status.is_empty() || grid.cols == 0 || grid.rows == 0 {
        return None;
    }
    let status_width = u16::try_from(status.chars().count()).ok()?;
    let required_cols = status_width.saturating_add(STATUS_RIGHT_MARGIN_COLS);
    if required_cols >= grid.cols {
        return None;
    }
    let row = status_overlay_row(grid, cursor, status_width, fallback)?;
    let col = grid
        .cols
        .saturating_sub(required_cols)
        .min(grid.cols.saturating_sub(status_width));
    let style = status_overlay_style();
    for (offset, ch) in status.chars().enumerate() {
        let offset = u16::try_from(offset).ok()?;
        let index = usize::from(row) * usize::from(grid.cols) + usize::from(col + offset);
        grid.cells[index] = CellSnapshot {
            text: ch.to_string(),
            style,
            hyperlink_id: 0,
            is_wide_leading: false,
            is_wide_trailing: false,
        };
    }
    Some(DirtyRegion {
        row,
        col,
        rows: 1,
        cols: status_width,
    })
}

fn overlay_target_is_blank(grid: &GridSnapshot, row: u16, col: u16, width: u16) -> bool {
    (0..width).all(|offset| grid.cell(row, col + offset).text.trim().is_empty())
}

fn status_overlay_row(
    grid: &GridSnapshot,
    cursor: CursorSnapshot,
    status_width: u16,
    fallback: bool,
) -> Option<u16> {
    let cursor_row = cursor.row.min(grid.rows.saturating_sub(1));
    let preferred = cursor_row.saturating_sub(1);
    let col = grid
        .cols
        .saturating_sub(status_width.saturating_add(STATUS_RIGHT_MARGIN_COLS))
        .min(grid.cols.saturating_sub(status_width));
    if overlay_target_is_blank(grid, preferred, col, status_width) {
        return Some(preferred);
    }
    if !fallback {
        return None;
    }
    if overlay_target_is_blank(grid, cursor_row, col, status_width) {
        return Some(cursor_row);
    }
    (cursor_row.saturating_add(1)..grid.rows)
        .chain((0..preferred).rev())
        .find(|row| overlay_target_is_blank(grid, *row, col, status_width))
}

fn status_overlay_style() -> Style {
    Style {
        foreground: Color::Ansi(14),
        bold: true,
        ..Style::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::{CursorShape, Terminal, TerminalConfig};

    use super::*;

    #[test]
    fn status_overlay_right_aligns_above_prompt_row() {
        let mut terminal = Terminal::new(TerminalConfig::new(24, 4).unwrap());
        terminal.write_str("one\r\n> ").unwrap();
        let mut grid = terminal.dump_grid();

        let region = apply_status_overlay(&mut grid, terminal.dump_cursor(), "144 fps").unwrap();

        assert_eq!(region.row, 0);
        assert_eq!(region.col, 16);
        assert_eq!(grid.line_text(0), "one             144 fps");
        assert_eq!(grid.cell(0, 16).style.foreground, Color::Ansi(14));
        assert_eq!(grid.line_text(1), ">");
    }

    #[test]
    fn status_overlay_uses_cursor_row_when_cursor_is_on_first_row() {
        let mut terminal = Terminal::new(TerminalConfig::new(16, 2).unwrap());
        terminal.write_str("> ").unwrap();
        let mut grid = terminal.dump_grid();
        let cursor = CursorSnapshot {
            row: 0,
            col: 2,
            visible: true,
            shape: CursorShape::Block,
            blinking: true,
        };

        let region = apply_status_overlay(&mut grid, cursor, "60 fps").unwrap();

        assert_eq!(region.row, 0);
        assert_eq!(grid.line_text(0), ">        60 fps");
    }

    #[test]
    fn status_overlay_skips_text_that_cannot_fit() {
        let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
        terminal.write_str("> ").unwrap();
        let mut grid = terminal.dump_grid();

        let region = apply_status_overlay(&mut grid, terminal.dump_cursor(), "144 fps");

        assert!(region.is_none());
        assert_eq!(grid.line_text(0), ">");
    }

    #[test]
    fn status_overlay_does_not_overwrite_shell_right_prompt() {
        let mut terminal = Terminal::new(TerminalConfig::new(24, 4).unwrap());
        terminal.write_str("left          12:34:56\r\n> ").unwrap();
        let mut grid = terminal.dump_grid();

        let region = apply_status_overlay(&mut grid, terminal.dump_cursor(), "144 fps");

        assert!(region.is_none());
        assert_eq!(grid.line_text(0), "left          12:34:56");
    }
}
