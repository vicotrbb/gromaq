//! Printable cell insertion and Unicode cluster assembly.

use crate::cell::Cell;

use super::Terminal;
use super::width::char_width;

mod cluster;

impl Terminal {
    pub(super) fn put_char(&mut self, ch: char) {
        let width = char_width(ch);
        if width == 0 {
            self.append_combining_mark(ch);
            return;
        }
        if self.append_emoji_modifier(ch) {
            return;
        }
        if self.append_regional_indicator_pair(ch) {
            return;
        }
        if self.append_zwj_joined_char(ch) {
            return;
        }
        let requested_width = if width == 2 { 2 } else { 1 };
        if self.auto_wrap
            && (self.wrap_pending || self.cursor.col + requested_width > self.config.cols)
        {
            self.wrap_pending = false;
            self.carriage_return();
            self.line_feed();
        }
        let span_width = if requested_width == 2 && self.cursor.col + 1 < self.config.cols {
            2
        } else {
            1
        };
        if self.insert_mode {
            self.insert_blank_chars(span_width);
        }
        self.clear_stale_wide_neighbors(self.cursor.row, self.cursor.col);
        let cell = self.grid.cell_mut(self.cursor.row, self.cursor.col);
        *cell = Cell {
            text: ch.to_string(),
            style: self.style,
            hyperlink_id: self.current_hyperlink_id,
            is_wide_leading: span_width == 2,
            is_wide_trailing: false,
        };
        self.mark_print_span(self.cursor.row, self.cursor.col, span_width);
        self.perf.dirty_cells += 1;
        if span_width == 2 {
            let trailing = self.grid.cell_mut(self.cursor.row, self.cursor.col + 1);
            *trailing = Cell {
                text: String::new(),
                style: self.style,
                hyperlink_id: self.current_hyperlink_id,
                is_wide_leading: false,
                is_wide_trailing: true,
            };
            self.perf.dirty_cells += 1;
        }
        if self.cursor.col + span_width >= self.config.cols {
            self.cursor.col = self.config.cols - 1;
            self.wrap_pending = self.auto_wrap;
        } else {
            self.cursor.col += span_width;
            self.wrap_pending = false;
        }
        self.last_printable_char = Some(ch);
    }

    fn clear_stale_wide_neighbors(&mut self, row: u16, col: u16) {
        if self.grid.cell(row, col).is_wide_trailing && col > 0 {
            self.flush_dirty_run();
            self.grid.clear_cell(row, col - 1, self.style);
            self.dirty.mark_cell(row, col - 1);
        }
        if col + 1 < self.config.cols && self.grid.cell(row, col + 1).is_wide_trailing {
            self.flush_dirty_run();
            self.grid.clear_cell(row, col + 1, self.style);
            self.dirty.mark_cell(row, col + 1);
        }
    }

    pub(super) fn repeat_last_printable_char(&mut self, count: u16) {
        if let Some(ch) = self.last_printable_char {
            for _ in 0..count {
                self.put_char(ch);
            }
        }
    }
}
