//! Cursor movement and tab-stop operations.

use super::{CursorShape, Terminal};

impl Terminal {
    pub(super) fn carriage_return(&mut self) {
        self.wrap_pending = false;
        self.cursor.col = 0;
    }

    pub(super) fn backspace(&mut self) {
        self.wrap_pending = false;
        self.cursor.col = self.cursor.col.saturating_sub(1);
    }

    pub(super) fn horizontal_tab(&mut self) {
        self.wrap_pending = false;
        self.cursor.col = self
            .tab_stops
            .iter()
            .enumerate()
            .skip(usize::from(self.cursor.col + 1))
            .find_map(|(col, enabled)| enabled.then_some(col as u16))
            .unwrap_or(self.config.cols - 1);
    }

    pub(super) fn set_horizontal_tab_stop(&mut self) {
        if let Some(tab_stop) = self.tab_stops.get_mut(usize::from(self.cursor.col)) {
            *tab_stop = true;
        }
    }

    pub(super) fn clear_tab_stop(&mut self, mode: u16) {
        match mode {
            0 => {
                if let Some(tab_stop) = self.tab_stops.get_mut(usize::from(self.cursor.col)) {
                    *tab_stop = false;
                }
            }
            3 => self.tab_stops.fill(false),
            _ => {}
        }
    }

    pub(super) fn move_cursor_forward_tabs(&mut self, count: u16) {
        for _ in 0..count {
            self.horizontal_tab();
        }
    }

    pub(super) fn move_cursor_backward_tabs(&mut self, count: u16) {
        self.wrap_pending = false;
        for _ in 0..count {
            self.cursor.col = self
                .tab_stops
                .iter()
                .enumerate()
                .take(usize::from(self.cursor.col))
                .rev()
                .find_map(|(col, enabled)| enabled.then_some(col as u16))
                .unwrap_or(0);
        }
    }

    pub(super) fn move_cursor_left(&mut self, count: u16) {
        self.wrap_pending = false;
        self.cursor.col = self.cursor.col.saturating_sub(count);
    }

    pub(super) fn move_cursor_right(&mut self, count: u16) {
        self.wrap_pending = false;
        self.cursor.col = (self.cursor.col + count).min(self.config.cols - 1);
    }

    pub(super) fn move_cursor_up(&mut self, count: u16) {
        self.wrap_pending = false;
        let (top, _) = self.vertical_cursor_bounds();
        self.cursor.row = self.cursor.row.saturating_sub(count).max(top);
    }

    pub(super) fn move_cursor_down(&mut self, count: u16) {
        self.wrap_pending = false;
        let (_, bottom) = self.vertical_cursor_bounds();
        self.cursor.row = self.cursor.row.saturating_add(count).min(bottom);
    }

    pub(super) fn move_cursor_next_line(&mut self, count: u16) {
        self.move_cursor_down(count);
        self.cursor.col = 0;
    }

    pub(super) fn move_cursor_previous_line(&mut self, count: u16) {
        self.move_cursor_up(count);
        self.cursor.col = 0;
    }

    pub(super) fn set_cursor_position(&mut self, row: u16, col: u16) {
        self.wrap_pending = false;
        self.cursor.row = self.absolute_cursor_row(row);
        self.cursor.col = col.saturating_sub(1).min(self.config.cols - 1);
    }

    pub(super) fn set_cursor_col(&mut self, col: u16) {
        self.wrap_pending = false;
        self.cursor.col = col.saturating_sub(1).min(self.config.cols - 1);
    }

    pub(super) fn set_cursor_row(&mut self, row: u16) {
        self.wrap_pending = false;
        self.cursor.row = self.absolute_cursor_row(row);
    }

    pub(super) fn set_cursor_shape(&mut self, shape: u16) {
        let Some((shape, blinking)) = (match shape {
            0 | 1 => Some((CursorShape::Block, true)),
            2 => Some((CursorShape::Block, false)),
            3 => Some((CursorShape::Underline, true)),
            4 => Some((CursorShape::Underline, false)),
            5 => Some((CursorShape::Bar, true)),
            6 => Some((CursorShape::Bar, false)),
            _ => None,
        }) else {
            return;
        };
        self.cursor.shape = shape;
        self.cursor.blinking = blinking;
    }

    fn absolute_cursor_row(&self, row: u16) -> u16 {
        let row = row.saturating_sub(1);
        if self.origin_mode {
            self.scroll_top.saturating_add(row).min(self.scroll_bottom)
        } else {
            row.min(self.config.rows - 1)
        }
    }

    fn vertical_cursor_bounds(&self) -> (u16, u16) {
        if self.origin_mode {
            (self.scroll_top, self.scroll_bottom)
        } else {
            (0, self.config.rows - 1)
        }
    }
}
