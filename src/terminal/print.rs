//! Printable cell insertion and Unicode cluster assembly.

use crate::cell::Cell;

use super::Terminal;
use super::width::{
    char_width, is_combining_enclosing_keycap, is_emoji_modifier, is_emoji_modifier_base_candidate,
    is_emoji_presentation_base_candidate, is_keycap_base_sequence, is_regional_indicator,
    is_variation_selector_16,
};

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

    fn append_emoji_modifier(&mut self, ch: char) -> bool {
        if !is_emoji_modifier(ch) {
            return false;
        }
        let Some((col, span_width)) = self.previous_visible_cell_with_span() else {
            return false;
        };
        if !self
            .grid
            .cell(self.cursor.row, col)
            .text
            .chars()
            .any(is_emoji_modifier_base_candidate)
        {
            return false;
        }

        let previous_last_printable = self.last_printable_char;
        self.append_to_previous_cluster(ch, col, span_width, span_width);
        self.last_printable_char = previous_last_printable;
        true
    }

    fn append_regional_indicator_pair(&mut self, ch: char) -> bool {
        if !is_regional_indicator(ch) {
            return false;
        }
        let Some((col, span_width)) = self.previous_visible_cell_with_span() else {
            return false;
        };
        let previous_text = &self.grid.cell(self.cursor.row, col).text;
        if previous_text
            .chars()
            .filter(|ch| is_regional_indicator(*ch))
            .count()
            != 1
            || previous_text.chars().count() != 1
        {
            return false;
        }

        self.append_to_previous_cluster(ch, col, span_width, 2);
        true
    }

    fn append_zwj_joined_char(&mut self, ch: char) -> bool {
        let Some((col, span_width)) = self.previous_visible_cell_with_span() else {
            return false;
        };
        let cell = self.grid.cell(self.cursor.row, col);
        if !cell.text.ends_with('\u{200d}') {
            return false;
        }

        let requested_span_width = if is_emoji_presentation_base_candidate(ch)
            || cell.text.chars().any(is_emoji_presentation_base_candidate)
        {
            2
        } else {
            span_width
        };
        self.append_to_previous_cluster(ch, col, span_width, requested_span_width);
        true
    }

    fn append_to_previous_cluster(
        &mut self,
        ch: char,
        col: u16,
        old_span_width: u16,
        requested_span_width: u16,
    ) {
        let span_width = if requested_span_width == 2 && col + 1 < self.config.cols {
            2
        } else {
            old_span_width
        };
        let trailing = {
            let cell = self.grid.cell_mut(self.cursor.row, col);
            cell.text.push(ch);
            cell.is_wide_leading = span_width == 2;
            cell.is_wide_trailing = false;
            Cell {
                text: String::new(),
                style: cell.style,
                hyperlink_id: cell.hyperlink_id,
                is_wide_leading: false,
                is_wide_trailing: true,
            }
        };
        if span_width == 2 && col + 1 < self.config.cols {
            *self.grid.cell_mut(self.cursor.row, col + 1) = trailing;
        }
        self.mark_print_span(self.cursor.row, col, span_width);
        self.perf.dirty_cells += u64::from(span_width);
        if span_width > old_span_width && self.cursor.col == col + old_span_width {
            if col + span_width >= self.config.cols {
                self.cursor.col = self.config.cols - 1;
                self.wrap_pending = self.auto_wrap;
            } else {
                self.cursor.col = col + span_width;
                self.wrap_pending = false;
            }
        }
        self.last_printable_char = Some(ch);
    }

    fn previous_visible_cell_with_span(&self) -> Option<(u16, u16)> {
        let mut col = if self.wrap_pending {
            self.cursor.col
        } else {
            if self.cursor.col == 0 {
                return None;
            }
            self.cursor.col - 1
        };
        if self.grid.cell(self.cursor.row, col).is_wide_trailing && col > 0 {
            col -= 1;
        }
        if self.grid.cell(self.cursor.row, col).is_wide_trailing {
            return None;
        }
        let span_width = if self.grid.cell(self.cursor.row, col).is_wide_leading {
            2
        } else {
            1
        };
        Some((col, span_width))
    }

    fn append_combining_mark(&mut self, ch: char) {
        let Some((col, span_width)) = self.previous_visible_cell_with_span() else {
            return;
        };
        let previous_last_printable = self.last_printable_char;
        let requested_span_width =
            if self.combining_mark_requests_emoji_width(ch, self.grid.cell(self.cursor.row, col)) {
                2
            } else {
                span_width
            };

        self.append_to_previous_cluster(ch, col, span_width, requested_span_width);
        self.last_printable_char = previous_last_printable;
    }

    fn combining_mark_requests_emoji_width(&self, ch: char, cell: &Cell) -> bool {
        if is_variation_selector_16(ch) {
            return cell.text.chars().any(is_emoji_presentation_base_candidate);
        }
        if is_combining_enclosing_keycap(ch) {
            return is_keycap_base_sequence(&cell.text);
        }
        false
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
