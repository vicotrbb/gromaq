//! Deterministic terminal state engine.

use vte::{Params, Parser, Perform};
use winit::keyboard::{Key, ModifiersState, PhysicalKey};

use crate::cell::{Cell, Color, Style, UnderlineStyle};
use crate::dirty::{DirtyRegion, DirtyTracker};
use crate::error::Result;
use crate::grid::Grid;
use crate::input::encode_winit_key_with_terminal_modes;
use crate::mouse::{MouseEvent, MouseReportState};
use crate::scrollback::Scrollback;
use crate::selection::SelectionRange;

mod modes;
mod osc;
mod params;
mod reflow;
mod reports;
mod selection_copy;
mod snapshot;
mod state;
mod types;
mod view;
mod width;

use osc::{
    Osc8HyperlinkAction, decode_bounded_osc_text, decode_osc8_hyperlink, decode_osc52_clipboard,
};
use params::{
    apply_grouped_sgr_param, default_tab_stops, first_value, first_values, grouped_extended_color,
    is_invalid_grouped_extended_color_param, parse_extended_color,
};
use snapshot::cell_screenshot_color;
use state::{CharacterSet, Cursor, DcsHandler, DirtyRun, SavedCursorState, SavedScreen};
pub use types::{CursorShape, CursorSnapshot, PerfSnapshot, Screenshot, TerminalConfig};
use width::{
    char_width, is_combining_enclosing_keycap, is_emoji_modifier, is_emoji_modifier_base_candidate,
    is_emoji_presentation_base_candidate, is_keycap_base_sequence, is_regional_indicator,
    is_variation_selector_16, map_dec_special_graphics, metadata_id_for_index,
};

const MAX_OSC_TITLE_BYTES: usize = 4096;
const MAX_OSC52_CLIPBOARD_BYTES: usize = 1_048_576;
const MAX_OSC8_HYPERLINK_BYTES: usize = 4096;
const MAX_METADATA_IDS: usize = 4096;
const MAX_OSC8_HYPERLINKS: usize = MAX_METADATA_IDS;
const MAX_UNDERLINE_COLORS: usize = MAX_METADATA_IDS;
const MAX_DCS_PAYLOAD_BYTES: usize = 64;

/// Deterministic terminal emulator state.
pub struct Terminal {
    config: TerminalConfig,
    grid: Grid,
    hard_breaks: Vec<bool>,
    tab_stops: Vec<bool>,
    scrollback: Scrollback,
    parser: Parser,
    cursor: Cursor,
    wrap_pending: bool,
    auto_wrap: bool,
    origin_mode: bool,
    application_cursor_keys: bool,
    application_keypad: bool,
    focus_event_reporting: bool,
    insert_mode: bool,
    linefeed_newline_mode: bool,
    g0_dec_special_graphics: bool,
    g1_dec_special_graphics: bool,
    active_charset: CharacterSet,
    scroll_top: u16,
    scroll_bottom: u16,
    saved_cursor: Option<Cursor>,
    saved_dec_cursor: Option<SavedCursorState>,
    saved_primary: Option<SavedScreen>,
    scrollback_view_offset: usize,
    saved_private_modes: Vec<(u16, bool)>,
    selection: Option<SelectionRange>,
    dirty: DirtyTracker,
    dirty_run: Option<DirtyRun>,
    mouse: MouseReportState,
    title: Option<String>,
    icon_label: Option<String>,
    clipboard_text: Option<String>,
    hyperlinks: Vec<String>,
    current_hyperlink_id: u16,
    underline_colors: Vec<Color>,
    bracketed_paste: bool,
    dcs_handler: Option<DcsHandler>,
    dcs_payload_overflowed: bool,
    dcs_payload: Vec<u8>,
    pending_response_bytes: Vec<u8>,
    style: Style,
    last_printable_char: Option<char>,
    perf: PerfSnapshot,
}

impl Terminal {
    /// Create a terminal with a blank grid.
    pub fn new(config: TerminalConfig) -> Self {
        Self {
            grid: Grid::new(config.cols, config.rows),
            hard_breaks: vec![false; usize::from(config.rows)],
            tab_stops: default_tab_stops(config.cols),
            scrollback: Scrollback::new(config.scrollback_limit),
            parser: Parser::new(),
            cursor: Cursor {
                row: 0,
                col: 0,
                visible: true,
                shape: CursorShape::Block,
                blinking: true,
            },
            wrap_pending: false,
            auto_wrap: true,
            origin_mode: false,
            application_cursor_keys: false,
            application_keypad: false,
            focus_event_reporting: false,
            insert_mode: false,
            linefeed_newline_mode: false,
            g0_dec_special_graphics: false,
            g1_dec_special_graphics: false,
            active_charset: CharacterSet::G0,
            scroll_top: 0,
            scroll_bottom: config.rows - 1,
            saved_cursor: None,
            saved_dec_cursor: None,
            saved_primary: None,
            scrollback_view_offset: 0,
            saved_private_modes: Vec::new(),
            selection: None,
            dirty: DirtyTracker::default(),
            dirty_run: None,
            mouse: MouseReportState::default(),
            title: None,
            icon_label: None,
            clipboard_text: None,
            hyperlinks: Vec::new(),
            current_hyperlink_id: 0,
            underline_colors: Vec::new(),
            bracketed_paste: false,
            dcs_handler: None,
            dcs_payload_overflowed: false,
            dcs_payload: Vec::new(),
            pending_response_bytes: Vec::new(),
            style: Style::default(),
            last_printable_char: None,
            perf: PerfSnapshot::default(),
            config,
        }
    }

    /// Feed UTF-8 text and escape sequences into the terminal parser.
    pub fn write_str(&mut self, input: &str) -> Result<()> {
        self.write_bytes(input.as_bytes())
    }

    /// Feed raw terminal bytes and escape sequences into the terminal parser.
    pub fn write_bytes(&mut self, input: &[u8]) -> Result<()> {
        if !input.is_empty() {
            self.scroll_display_to_bottom();
        }
        let mut parser = std::mem::take(&mut self.parser);
        parser.advance(self, input);
        self.parser = parser;
        self.flush_dirty_run();
        self.perf.parsed_bytes += input.len() as u64;
        Ok(())
    }

    /// Resize the visible grid while preserving top-left content.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.resize_with_pixel_size(
            cols,
            rows,
            self.config.pixel_width,
            self.config.pixel_height,
        )
    }

    /// Resize the visible grid and update native pixel dimensions.
    pub fn resize_with_pixel_size(
        &mut self,
        cols: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> Result<()> {
        let config = TerminalConfig {
            cols,
            rows,
            pixel_width,
            pixel_height,
            scrollback_limit: self.config.scrollback_limit,
        }
        .validate()?;
        self.reconfigure(config)
    }

    /// Reconfigure terminal dimensions, pixel size, and scrollback retention.
    pub fn reconfigure(&mut self, config: TerminalConfig) -> Result<()> {
        let config = config.validate()?;
        self.flush_dirty_run();
        self.scrollback.set_limit(config.scrollback_limit);
        self.scrollback.reflow(self.config.cols, config.cols);
        let (grid, hard_breaks) = self.reflow_visible_grid(config.cols, config.rows);
        self.grid = grid;
        self.hard_breaks = hard_breaks;
        if let Some(saved) = &mut self.saved_primary {
            let (grid, hard_breaks) =
                reflow::reflow_grid(&saved.grid, &saved.hard_breaks, config.cols, config.rows);
            saved.grid = grid;
            saved.hard_breaks = hard_breaks;
            saved.tab_stops = default_tab_stops(config.cols);
            saved.scroll_top = 0;
            saved.scroll_bottom = config.rows - 1;
            saved.cursor.row = saved.cursor.row.min(config.rows - 1);
            saved.cursor.col = saved.cursor.col.min(config.cols - 1);
            saved.wrap_pending = false;
        }
        self.tab_stops = default_tab_stops(config.cols);
        self.scroll_top = 0;
        self.scroll_bottom = config.rows - 1;
        self.cursor.row = self.cursor.row.min(config.rows - 1);
        self.cursor.col = self.cursor.col.min(config.cols - 1);
        self.wrap_pending = false;
        self.scrollback_view_offset = 0;
        self.selection = None;
        self.dirty.mark_viewport(config.rows, config.cols);
        self.config = config;
        self.perf.resizes += 1;
        Ok(())
    }

    /// Return performance counters.
    pub fn dump_perf_metrics(&self) -> PerfSnapshot {
        self.perf
    }

    /// Drain pending dirty regions for renderer scheduling.
    pub fn take_dirty_regions(&mut self) -> Vec<DirtyRegion> {
        self.flush_dirty_run();
        let regions = self.dirty.take();
        if !regions.is_empty() {
            self.perf.dirty_region_batches += 1;
        }
        regions
    }

    /// Mark the full visible viewport dirty for the next renderer pass.
    pub fn invalidate_viewport(&mut self) {
        self.flush_dirty_run();
        self.dirty.mark_viewport(self.config.rows, self.config.cols);
    }

    /// Encode a mouse event for the running application when reporting is enabled.
    pub fn encode_mouse_event(&self, event: MouseEvent) -> Option<Vec<u8>> {
        self.mouse.encode(event)
    }

    /// Encode a native logical key according to terminal input modes.
    pub fn encode_winit_key_input(&self, key: &Key, modifiers: ModifiersState) -> Option<Vec<u8>> {
        self.encode_winit_key_event_input(key, None, modifiers)
    }

    /// Encode a native key event according to terminal input modes.
    pub fn encode_winit_key_event_input(
        &self,
        key: &Key,
        physical_key: Option<PhysicalKey>,
        modifiers: ModifiersState,
    ) -> Option<Vec<u8>> {
        encode_winit_key_with_terminal_modes(
            key,
            physical_key,
            modifiers,
            self.application_cursor_keys,
            self.application_keypad,
        )
    }

    /// Encode a terminal focus event when focus reporting mode is enabled.
    pub fn encode_focus_event(&self, focused: bool) -> Option<Vec<u8>> {
        if !self.focus_event_reporting {
            return None;
        }
        Some(if focused {
            b"\x1b[I".to_vec()
        } else {
            b"\x1b[O".to_vec()
        })
    }

    /// Return the current window title set by OSC 0 or OSC 2.
    pub fn dump_title(&self) -> Option<String> {
        self.title.clone()
    }

    /// Return clipboard text accepted from terminal control sequences.
    pub fn dump_clipboard_text(&self) -> Option<String> {
        self.clipboard_text.clone()
    }

    /// Encode pasted text for the running application.
    pub fn encode_paste_text(&self, text: &str) -> Vec<u8> {
        if self.bracketed_paste {
            let mut bytes = Vec::with_capacity(text.len() + b"\x1b[200~\x1b[201~".len());
            bytes.extend_from_slice(b"\x1b[200~");
            bytes.extend_from_slice(text.as_bytes());
            bytes.extend_from_slice(b"\x1b[201~");
            bytes
        } else {
            text.as_bytes().to_vec()
        }
    }

    /// Drain terminal-generated response bytes that should be written back to the PTY.
    pub fn take_pending_response_bytes(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending_response_bytes)
    }

    /// Return a deterministic one-pixel-per-cell RGBA screenshot of the visible grid.
    pub fn screenshot(&self) -> Screenshot {
        let width = u32::from(self.config.cols);
        let height = u32::from(self.config.rows);
        let mut rgba =
            Vec::with_capacity(usize::from(self.config.cols) * usize::from(self.config.rows) * 4);
        let grid = self.dump_grid();
        let cursor = self.dump_cursor();
        for row in 0..self.config.rows {
            for col in 0..self.config.cols {
                let color = if cursor.visible && cursor.row == row && cursor.col == col {
                    [64, 160, 255, 255]
                } else {
                    cell_screenshot_color(grid.cell(row, col))
                };
                rgba.extend_from_slice(&color);
            }
        }
        Screenshot {
            width,
            height,
            rgba,
        }
    }

    fn put_char(&mut self, ch: char) {
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

    fn line_feed(&mut self) {
        self.flush_dirty_run();
        if self.cursor.row == self.scroll_bottom {
            self.scroll_region_up(1);
        } else if self.cursor.row + 1 < self.config.rows {
            self.cursor.row += 1;
        }
    }

    fn index(&mut self) {
        self.wrap_pending = false;
        self.line_feed();
    }

    fn next_line(&mut self) {
        self.wrap_pending = false;
        self.line_feed();
        self.carriage_return();
    }

    fn scroll_region_up(&mut self, count: u16) {
        let region_rows = self.scroll_bottom - self.scroll_top + 1;
        if self.scroll_top == 0 && self.scroll_bottom == self.config.rows - 1 && count == 1 {
            let removed = self.trimmed_visible_row_snapshot(0);
            let removed_hard_break = self.hard_breaks.first().copied().unwrap_or(false);
            self.grid.scroll_up_one(self.style);
            if self.saved_primary.is_none() {
                self.scrollback
                    .push_cells_with_hard_break(removed, removed_hard_break);
            }
            if !self.hard_breaks.is_empty() {
                self.hard_breaks.rotate_left(1);
                if let Some(last) = self.hard_breaks.last_mut() {
                    *last = false;
                }
            }
            self.dirty.mark_viewport(self.config.rows, self.config.cols);
            self.perf.dirty_cells += u64::from(self.config.rows) * u64::from(self.config.cols);
            self.perf.scrolls += 1;
        } else {
            let count = count.min(region_rows);
            self.grid
                .scroll_region_up(self.scroll_top, self.scroll_bottom, count, self.style);
            self.delete_hard_break_rows_in_region(self.scroll_top, self.scroll_bottom, count);
            self.dirty.mark_region(DirtyRegion {
                row: self.scroll_top,
                col: 0,
                rows: region_rows,
                cols: self.config.cols,
            });
            self.perf.dirty_cells += u64::from(region_rows) * u64::from(self.config.cols);
            self.perf.scrolls += u64::from(count);
        }
    }

    fn reverse_index(&mut self) {
        self.flush_dirty_run();
        self.wrap_pending = false;
        if self.cursor.row == self.scroll_top {
            self.scroll_region_down(1);
        } else {
            self.cursor.row = self.cursor.row.saturating_sub(1);
        }
    }

    fn scroll_region_down(&mut self, count: u16) {
        let region_rows = self.scroll_bottom - self.scroll_top + 1;
        let count = count.min(region_rows);
        self.grid
            .scroll_region_down(self.scroll_top, self.scroll_bottom, count, self.style);
        self.insert_hard_break_rows_in_region(self.scroll_top, self.scroll_bottom, count);
        self.dirty.mark_region(DirtyRegion {
            row: self.scroll_top,
            col: 0,
            rows: region_rows,
            cols: self.config.cols,
        });
        self.perf.dirty_cells += u64::from(region_rows) * u64::from(self.config.cols);
        self.perf.scrolls += u64::from(count);
    }

    fn carriage_return(&mut self) {
        self.wrap_pending = false;
        self.cursor.col = 0;
    }

    fn backspace(&mut self) {
        self.wrap_pending = false;
        self.cursor.col = self.cursor.col.saturating_sub(1);
    }

    fn horizontal_tab(&mut self) {
        self.wrap_pending = false;
        self.cursor.col = self
            .tab_stops
            .iter()
            .enumerate()
            .skip(usize::from(self.cursor.col + 1))
            .find_map(|(col, enabled)| enabled.then_some(col as u16))
            .unwrap_or(self.config.cols - 1);
    }

    fn set_horizontal_tab_stop(&mut self) {
        if let Some(tab_stop) = self.tab_stops.get_mut(usize::from(self.cursor.col)) {
            *tab_stop = true;
        }
    }

    fn clear_tab_stop(&mut self, mode: u16) {
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

    fn move_cursor_forward_tabs(&mut self, count: u16) {
        for _ in 0..count {
            self.horizontal_tab();
        }
    }

    fn move_cursor_backward_tabs(&mut self, count: u16) {
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

    fn move_cursor_left(&mut self, count: u16) {
        self.wrap_pending = false;
        self.cursor.col = self.cursor.col.saturating_sub(count);
    }

    fn move_cursor_right(&mut self, count: u16) {
        self.wrap_pending = false;
        self.cursor.col = (self.cursor.col + count).min(self.config.cols - 1);
    }

    fn move_cursor_up(&mut self, count: u16) {
        self.wrap_pending = false;
        let (top, _) = self.vertical_cursor_bounds();
        self.cursor.row = self.cursor.row.saturating_sub(count).max(top);
    }

    fn move_cursor_down(&mut self, count: u16) {
        self.wrap_pending = false;
        let (_, bottom) = self.vertical_cursor_bounds();
        self.cursor.row = self.cursor.row.saturating_add(count).min(bottom);
    }

    fn move_cursor_next_line(&mut self, count: u16) {
        self.move_cursor_down(count);
        self.cursor.col = 0;
    }

    fn move_cursor_previous_line(&mut self, count: u16) {
        self.move_cursor_up(count);
        self.cursor.col = 0;
    }

    fn set_cursor_position(&mut self, row: u16, col: u16) {
        self.wrap_pending = false;
        self.cursor.row = self.absolute_cursor_row(row);
        self.cursor.col = col.saturating_sub(1).min(self.config.cols - 1);
    }

    fn set_cursor_col(&mut self, col: u16) {
        self.wrap_pending = false;
        self.cursor.col = col.saturating_sub(1).min(self.config.cols - 1);
    }

    fn set_cursor_row(&mut self, row: u16) {
        self.wrap_pending = false;
        self.cursor.row = self.absolute_cursor_row(row);
    }

    fn set_cursor_shape(&mut self, shape: u16) {
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

    fn erase_line(&mut self, mode: u16) {
        self.flush_dirty_run();
        match mode {
            0 => {
                for col in self.cursor.col..self.config.cols {
                    self.grid.clear_cell(self.cursor.row, col, self.style);
                }
                let cols = self.config.cols - self.cursor.col;
                self.dirty.mark_span(self.cursor.row, self.cursor.col, cols);
                self.perf.dirty_cells += u64::from(cols);
                if self.cursor.col == 0 {
                    self.hard_breaks[usize::from(self.cursor.row)] = false;
                }
            }
            1 => {
                for col in 0..=self.cursor.col {
                    self.grid.clear_cell(self.cursor.row, col, self.style);
                }
                let cols = self.cursor.col + 1;
                self.dirty.mark_span(self.cursor.row, 0, cols);
                self.perf.dirty_cells += u64::from(cols);
                if self.cursor.col + 1 == self.config.cols {
                    self.hard_breaks[usize::from(self.cursor.row)] = false;
                }
            }
            2 => {
                self.grid.clear_row(self.cursor.row, self.style);
                self.hard_breaks[usize::from(self.cursor.row)] = false;
                self.dirty.mark_span(self.cursor.row, 0, self.config.cols);
                self.perf.dirty_cells += u64::from(self.config.cols);
            }
            _ => {}
        }
    }

    fn erase_display(&mut self, mode: u16) {
        self.flush_dirty_run();
        match mode {
            0 => {
                for row in self.cursor.row..self.config.rows {
                    let start_col = if row == self.cursor.row {
                        self.cursor.col
                    } else {
                        0
                    };
                    for col in start_col..self.config.cols {
                        self.grid.clear_cell(row, col, self.style);
                    }
                    let cols = self.config.cols - start_col;
                    self.dirty.mark_span(row, start_col, cols);
                    self.perf.dirty_cells += u64::from(cols);
                    if start_col == 0 {
                        self.hard_breaks[usize::from(row)] = false;
                    }
                }
            }
            1 => {
                for row in 0..=self.cursor.row {
                    let end_col = if row == self.cursor.row {
                        self.cursor.col
                    } else {
                        self.config.cols - 1
                    };
                    for col in 0..=end_col {
                        self.grid.clear_cell(row, col, self.style);
                    }
                    let cols = end_col + 1;
                    self.dirty.mark_span(row, 0, cols);
                    self.perf.dirty_cells += u64::from(cols);
                    if end_col + 1 == self.config.cols {
                        self.hard_breaks[usize::from(row)] = false;
                    }
                }
            }
            2 => {
                for row in 0..self.config.rows {
                    self.grid.clear_row(row, self.style);
                }
                self.hard_breaks.fill(false);
                self.wrap_pending = false;
                self.perf.dirty_cells += u64::from(self.config.cols) * u64::from(self.config.rows);
                self.dirty.mark_viewport(self.config.rows, self.config.cols);
            }
            3 => self.scrollback.clear(),
            _ => {}
        }
    }

    #[cold]
    #[inline(never)]
    fn screen_alignment_test(&mut self) {
        self.flush_dirty_run();
        self.wrap_pending = false;
        for row in 0..self.config.rows {
            for col in 0..self.config.cols {
                *self.grid.cell_mut(row, col) = Cell {
                    text: "E".to_owned(),
                    style: self.style,
                    hyperlink_id: 0,
                    is_wide_leading: false,
                    is_wide_trailing: false,
                };
            }
            self.hard_breaks[usize::from(row)] = false;
        }
        self.perf.dirty_cells += u64::from(self.config.cols) * u64::from(self.config.rows);
        self.dirty.mark_viewport(self.config.rows, self.config.cols);
    }

    fn insert_blank_chars(&mut self, count: u16) {
        self.flush_dirty_run();
        self.grid
            .insert_blank_cells(self.cursor.row, self.cursor.col, count, self.style);
        let repaired = self
            .grid
            .repair_wide_cells_in_row(self.cursor.row, self.style);
        self.mark_edit_span_dirty(self.cursor.col, self.config.cols, repaired);
    }

    fn delete_chars(&mut self, count: u16) {
        self.flush_dirty_run();
        self.grid
            .delete_cells(self.cursor.row, self.cursor.col, count, self.style);
        let repaired = self
            .grid
            .repair_wide_cells_in_row(self.cursor.row, self.style);
        self.mark_edit_span_dirty(self.cursor.col, self.config.cols, repaired);
    }

    fn erase_chars(&mut self, count: u16) {
        self.flush_dirty_run();
        let count = count.min(self.config.cols - self.cursor.col);
        for col in self.cursor.col..self.cursor.col + count {
            self.grid.clear_cell(self.cursor.row, col, self.style);
        }
        let repaired = self
            .grid
            .repair_wide_cells_in_row(self.cursor.row, self.style);
        self.mark_edit_span_dirty(self.cursor.col, self.cursor.col + count, repaired);
    }

    fn mark_edit_span_dirty(
        &mut self,
        edit_start: u16,
        edit_end: u16,
        repaired: Option<(u16, u16)>,
    ) {
        let (start, end) = match repaired {
            Some((repair_start, repair_end)) => {
                (edit_start.min(repair_start), edit_end.max(repair_end))
            }
            None => (edit_start, edit_end),
        };
        let cols = end.saturating_sub(start);
        self.dirty.mark_span(self.cursor.row, start, cols);
        self.perf.dirty_cells += u64::from(cols);
    }

    fn repeat_last_printable_char(&mut self, count: u16) {
        if let Some(ch) = self.last_printable_char {
            for _ in 0..count {
                self.put_char(ch);
            }
        }
    }

    fn insert_blank_lines(&mut self, count: u16) {
        self.flush_dirty_run();
        if self.cursor.row < self.scroll_top || self.cursor.row > self.scroll_bottom {
            return;
        }
        let bottom = self.scroll_bottom;
        self.grid
            .insert_blank_rows_in_region(self.cursor.row, bottom, count, self.style);
        self.insert_hard_break_rows_in_region(self.cursor.row, bottom, count);
        let rows = bottom - self.cursor.row + 1;
        self.dirty.mark_region(DirtyRegion {
            row: self.cursor.row,
            col: 0,
            rows,
            cols: self.config.cols,
        });
        self.perf.dirty_cells += u64::from(rows) * u64::from(self.config.cols);
    }

    fn delete_lines(&mut self, count: u16) {
        self.flush_dirty_run();
        if self.cursor.row < self.scroll_top || self.cursor.row > self.scroll_bottom {
            return;
        }
        let bottom = self.scroll_bottom;
        self.grid
            .delete_rows_in_region(self.cursor.row, bottom, count, self.style);
        self.delete_hard_break_rows_in_region(self.cursor.row, bottom, count);
        let rows = bottom - self.cursor.row + 1;
        self.dirty.mark_region(DirtyRegion {
            row: self.cursor.row,
            col: 0,
            rows,
            cols: self.config.cols,
        });
        self.perf.dirty_cells += u64::from(rows) * u64::from(self.config.cols);
    }

    fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        let top = top.saturating_sub(1);
        let bottom = bottom.saturating_sub(1).min(self.config.rows - 1);
        if top >= bottom {
            return;
        }
        self.flush_dirty_run();
        self.scroll_top = top;
        self.scroll_bottom = bottom;
        self.cursor.row = if self.origin_mode { self.scroll_top } else { 0 };
        self.cursor.col = 0;
        self.wrap_pending = false;
    }

    fn scroll_viewport_up(&mut self, count: u16) {
        self.flush_dirty_run();
        self.wrap_pending = false;
        self.scroll_region_up(count);
    }

    fn scroll_viewport_down(&mut self, count: u16) {
        self.flush_dirty_run();
        self.wrap_pending = false;
        self.scroll_region_down(count);
    }

    fn save_cursor(&mut self) {
        self.saved_cursor = Some(self.cursor);
    }

    fn restore_cursor(&mut self) {
        if let Some(saved) = self.saved_cursor {
            self.cursor = saved;
        }
    }

    #[cold]
    #[inline(never)]
    fn soft_reset(&mut self) {
        self.wrap_pending = false;
        self.auto_wrap = false;
        self.origin_mode = false;
        self.application_cursor_keys = false;
        self.application_keypad = false;
        self.insert_mode = false;
        self.linefeed_newline_mode = false;
        self.cursor.visible = true;
        self.g0_dec_special_graphics = false;
        self.g1_dec_special_graphics = false;
        self.active_charset = CharacterSet::G0;
        self.scroll_top = 0;
        self.scroll_bottom = self.config.rows - 1;
        self.style = Style::default();
        self.saved_dec_cursor = Some(SavedCursorState {
            cursor: Cursor {
                row: 0,
                col: 0,
                visible: true,
                shape: CursorShape::Block,
                blinking: true,
            },
            style: Style::default(),
            g0_dec_special_graphics: false,
            g1_dec_special_graphics: false,
            active_charset: CharacterSet::G0,
        });
    }

    fn reset_to_initial_state(&mut self) {
        self.flush_dirty_run();
        self.grid = Grid::new(self.config.cols, self.config.rows);
        self.hard_breaks = vec![false; usize::from(self.config.rows)];
        self.tab_stops = default_tab_stops(self.config.cols);
        self.scrollback = Scrollback::new(self.config.scrollback_limit);
        self.cursor = Cursor {
            row: 0,
            col: 0,
            visible: true,
            shape: CursorShape::Block,
            blinking: true,
        };
        self.wrap_pending = false;
        self.auto_wrap = true;
        self.origin_mode = false;
        self.application_cursor_keys = false;
        self.application_keypad = false;
        self.focus_event_reporting = false;
        self.insert_mode = false;
        self.linefeed_newline_mode = false;
        self.g0_dec_special_graphics = false;
        self.g1_dec_special_graphics = false;
        self.active_charset = CharacterSet::G0;
        self.scroll_top = 0;
        self.scroll_bottom = self.config.rows - 1;
        self.saved_cursor = None;
        self.saved_dec_cursor = None;
        self.saved_primary = None;
        self.scrollback_view_offset = 0;
        self.saved_private_modes.clear();
        self.selection = None;
        self.dirty = DirtyTracker::default();
        self.dirty_run = None;
        self.mouse = MouseReportState::default();
        self.title = None;
        self.icon_label = None;
        self.clipboard_text = None;
        self.hyperlinks.clear();
        self.current_hyperlink_id = 0;
        self.underline_colors.clear();
        self.bracketed_paste = false;
        self.dcs_handler = None;
        self.dcs_payload_overflowed = false;
        self.dcs_payload.clear();
        self.pending_response_bytes.clear();
        self.style = Style::default();
        self.last_printable_char = None;
        self.dirty.mark_viewport(self.config.rows, self.config.cols);
        self.perf.dirty_cells += u64::from(self.config.rows) * u64::from(self.config.cols);
    }

    fn apply_sgr(&mut self, params: &Params) {
        if params.is_empty() {
            self.style = Style::default();
            return;
        }
        let mut flattened = Vec::new();
        for param in params.iter() {
            if param.is_empty() {
                flattened.push(0);
                continue;
            }
            if apply_grouped_sgr_param(&mut self.style, param) {
                continue;
            }
            if self.apply_grouped_extended_color_param(param) {
                continue;
            }
            if is_invalid_grouped_extended_color_param(param) {
                continue;
            }
            flattened.extend(param.iter().copied());
        }
        self.apply_flat_sgr(flattened);
    }

    fn apply_flat_sgr(&mut self, params: Vec<u16>) {
        let mut iter = params.into_iter().peekable();
        while let Some(param) = iter.next() {
            match param {
                0 => self.style = Style::default(),
                1 => self.style.bold = true,
                2 => self.style.dim = true,
                3 => self.style.italic = true,
                4 => self.style.underline = true,
                21 => {
                    self.style.underline = true;
                    self.style.underline_style = UnderlineStyle::Double;
                }
                5 | 6 => self.style.blink = true,
                7 => self.style.inverse = true,
                8 => self.style.hidden = true,
                9 => self.style.strikethrough = true,
                22 => {
                    self.style.bold = false;
                    self.style.dim = false;
                }
                23 => self.style.italic = false,
                24 => {
                    self.style.underline = false;
                    self.style.underline_style = UnderlineStyle::Single;
                }
                25 => self.style.blink = false,
                27 => self.style.inverse = false,
                28 => self.style.hidden = false,
                29 => self.style.strikethrough = false,
                51 => {
                    self.style.framed = true;
                    self.style.encircled = false;
                }
                52 => {
                    self.style.framed = false;
                    self.style.encircled = true;
                }
                53 => self.style.overline = true,
                54 => {
                    self.style.framed = false;
                    self.style.encircled = false;
                }
                55 => self.style.overline = false,
                59 => self.style.underline_color_id = 0,
                30..=37 => self.style.foreground = Color::Ansi((param - 30) as u8),
                39 => self.style.foreground = Color::Default,
                40..=47 => self.style.background = Color::Ansi((param - 40) as u8),
                49 => self.style.background = Color::Default,
                90..=97 => self.style.foreground = Color::Ansi((param - 90 + 8) as u8),
                100..=107 => self.style.background = Color::Ansi((param - 100 + 8) as u8),
                38 | 48 | 58 => {
                    if let Some(color) = parse_extended_color(&mut iter) {
                        self.apply_extended_color_target(param, color);
                    }
                }
                _ => {}
            }
        }
    }

    fn apply_grouped_extended_color_param(&mut self, param: &[u16]) -> bool {
        let Some((target, color)) = grouped_extended_color(param) else {
            return false;
        };
        self.apply_extended_color_target(target, color)
    }

    fn apply_extended_color_target(&mut self, target: u16, color: Color) -> bool {
        match target {
            38 => self.style.foreground = color,
            48 => self.style.background = color,
            58 => self.style.underline_color_id = self.intern_underline_color(color),
            _ => return false,
        }
        true
    }

    fn intern_hyperlink(&mut self, uri: String) -> u16 {
        if let Some(index) = self.hyperlinks.iter().position(|existing| existing == &uri) {
            return metadata_id_for_index(index);
        }
        if self.hyperlinks.len() == MAX_OSC8_HYPERLINKS {
            return 0;
        }
        self.hyperlinks.push(uri);
        metadata_id_for_index(self.hyperlinks.len() - 1)
    }

    fn intern_underline_color(&mut self, color: Color) -> u16 {
        if color == Color::Default {
            return 0;
        }
        if let Some(index) = self
            .underline_colors
            .iter()
            .position(|existing| *existing == color)
        {
            return metadata_id_for_index(index);
        }
        if self.underline_colors.len() == MAX_UNDERLINE_COLORS {
            return 0;
        }
        self.underline_colors.push(color);
        metadata_id_for_index(self.underline_colors.len() - 1)
    }

    fn mark_print_span(&mut self, row: u16, col: u16, cols: u16) {
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

    fn flush_dirty_run(&mut self) {
        if let Some(run) = self.dirty_run.take() {
            self.dirty
                .mark_span(run.row, run.col_start, run.col_end - run.col_start);
        }
    }

    fn reflow_visible_grid(&self, cols: u16, rows: u16) -> (Grid, Vec<bool>) {
        reflow::reflow_grid(&self.grid, &self.hard_breaks, cols, rows)
    }

    fn delete_hard_break_rows_in_region(&mut self, top: u16, bottom: u16, count: u16) {
        if top >= self.config.rows || bottom >= self.config.rows || top >= bottom || count == 0 {
            return;
        }
        let height = bottom - top + 1;
        let count = count.min(height);
        for target_row in top..=bottom - count {
            self.hard_breaks[usize::from(target_row)] =
                self.hard_breaks[usize::from(target_row + count)];
        }
        for blank_row in bottom - count + 1..=bottom {
            self.hard_breaks[usize::from(blank_row)] = false;
        }
    }

    fn insert_hard_break_rows_in_region(&mut self, top: u16, bottom: u16, count: u16) {
        if top >= self.config.rows || bottom >= self.config.rows || top >= bottom || count == 0 {
            return;
        }
        let height = bottom - top + 1;
        let count = count.min(height);
        for target_row in (top + count..=bottom).rev() {
            self.hard_breaks[usize::from(target_row)] =
                self.hard_breaks[usize::from(target_row - count)];
        }
        for blank_row in top..top + count {
            self.hard_breaks[usize::from(blank_row)] = false;
        }
    }
}

impl Perform for Terminal {
    fn hook(&mut self, _params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        self.dcs_payload.clear();
        self.dcs_payload_overflowed = false;
        self.dcs_handler = if !ignore && intermediates == b"$" && action == 'q' {
            Some(DcsHandler::Decrqss)
        } else {
            None
        };
    }

    fn put(&mut self, byte: u8) {
        if self.dcs_handler.is_none() {
            self.dcs_payload.clear();
            return;
        }
        if self.dcs_payload_overflowed {
            return;
        }
        if self.dcs_payload.len() == MAX_DCS_PAYLOAD_BYTES {
            self.dcs_payload_overflowed = true;
            self.dcs_payload.clear();
            return;
        }
        self.dcs_payload.push(byte);
    }

    fn unhook(&mut self) {
        let Some(DcsHandler::Decrqss) = self.dcs_handler.take() else {
            self.dcs_payload.clear();
            self.dcs_payload_overflowed = false;
            return;
        };
        if self.dcs_payload_overflowed {
            self.dcs_payload.clear();
            self.dcs_payload_overflowed = false;
            self.report_decrqss(&[]);
        } else {
            let request = std::mem::take(&mut self.dcs_payload);
            self.report_decrqss(&request);
        }
    }

    fn print(&mut self, c: char) {
        let dec_special_graphics = match self.active_charset {
            CharacterSet::G0 => self.g0_dec_special_graphics,
            CharacterSet::G1 => self.g1_dec_special_graphics,
        };
        let ch = if dec_special_graphics {
            map_dec_special_graphics(c)
        } else {
            c
        };
        self.put_char(ch);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0b | 0x0c => {
                if let Some(hard_break) = self.hard_breaks.get_mut(usize::from(self.cursor.row)) {
                    *hard_break = true;
                }
                self.wrap_pending = false;
                self.line_feed();
                if self.linefeed_newline_mode {
                    self.carriage_return();
                }
            }
            b'\r' => self.carriage_return(),
            0x08 => self.backspace(),
            b'\t' => self.horizontal_tab(),
            0x0e => self.active_charset = CharacterSet::G1,
            0x0f => self.active_charset = CharacterSet::G0,
            0x84 => self.index(),
            0x85 => self.next_line(),
            0x88 => self.set_horizontal_tab_stop(),
            0x8d => self.reverse_index(),
            0x9a => self.report_decid(),
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        if ignore {
            return;
        }
        let first = first_value(params, 0).unwrap_or(0);
        let count = if first == 0 { 1 } else { first };
        match action {
            'A' => self.move_cursor_up(count),
            'B' => self.move_cursor_down(count),
            'C' => self.move_cursor_right(count),
            'D' => self.move_cursor_left(count),
            'E' => self.move_cursor_next_line(count),
            'F' => self.move_cursor_previous_line(count),
            'I' => self.move_cursor_forward_tabs(count),
            'Z' => self.move_cursor_backward_tabs(count),
            '@' => self.insert_blank_chars(count),
            'b' => self.repeat_last_printable_char(count),
            'P' => self.delete_chars(count),
            'X' => self.erase_chars(count),
            'c' if intermediates.is_empty() => self.report_primary_device_attributes(first),
            'c' if intermediates == b">" => self.report_secondary_device_attributes(first),
            'L' => self.insert_blank_lines(count),
            'M' => self.delete_lines(count),
            'S' => self.scroll_viewport_up(count),
            'T' => self.scroll_viewport_down(count),
            '^' => self.scroll_viewport_down(count),
            '`' => self.set_cursor_col(count),
            'a' => self.move_cursor_right(count),
            'H' | 'f' => {
                let row = first_value(params, 0).unwrap_or(1);
                let col = first_value(params, 1).unwrap_or(1);
                self.set_cursor_position(row, col);
            }
            'G' => self.set_cursor_col(count),
            'g' => self.clear_tab_stop(first),
            'd' => self.set_cursor_row(count),
            'e' => self.move_cursor_down(count),
            'J' => self.erase_display(first),
            'K' => self.erase_line(first),
            'm' => self.apply_sgr(params),
            'n' if intermediates.is_empty() => self.report_device_status(first),
            'n' if intermediates == b"?" => self.report_private_device_status(first),
            'p' if intermediates == b"$" => self.report_mode_state(false, first),
            'p' if intermediates == b"?$" => self.report_mode_state(true, first),
            'p' if intermediates == b"!" => self.soft_reset(),
            'q' if intermediates == b" " => self.set_cursor_shape(first),
            'r' if intermediates == b"?" => self.restore_private_modes(first_values(params)),
            'r' => {
                let top = first_value(params, 0).unwrap_or(1);
                let bottom = first_value(params, 1).unwrap_or(self.config.rows);
                self.set_scroll_region(top, bottom);
            }
            's' if intermediates == b"?" => self.save_private_modes(first_values(params)),
            's' => self.save_cursor(),
            't' if intermediates.is_empty() => self.report_window_manipulation(first),
            'u' => self.restore_cursor(),
            'x' if intermediates.is_empty() => self.report_terminal_parameters(first),
            'h' if intermediates.is_empty() => {
                for mode in first_values(params) {
                    self.set_mode(mode, true);
                }
            }
            'l' if intermediates.is_empty() => {
                for mode in first_values(params) {
                    self.set_mode(mode, false);
                }
            }
            'h' if intermediates == b"?" => {
                for mode in first_values(params) {
                    self.set_private_mode(mode, true);
                }
            }
            'l' if intermediates == b"?" => {
                for mode in first_values(params) {
                    self.set_private_mode(mode, false);
                }
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        if ignore {
            return;
        }
        match (intermediates, byte) {
            (b"(", b'0') => self.g0_dec_special_graphics = true,
            (b"(", b'B') => self.g0_dec_special_graphics = false,
            (b"(", b'A') => self.g0_dec_special_graphics = false,
            (b"(", b'U') => self.g0_dec_special_graphics = false,
            (b")", b'0') => self.g1_dec_special_graphics = true,
            (b")", b'B') => self.g1_dec_special_graphics = false,
            (b")", b'A') => self.g1_dec_special_graphics = false,
            (b")", b'U') => self.g1_dec_special_graphics = false,
            (b"", b'D') => self.index(),
            (b"", b'E') => self.next_line(),
            (b"", b'H') => self.set_horizontal_tab_stop(),
            (b"", b'M') => self.reverse_index(),
            (b"", b'=') => self.application_keypad = true,
            (b"", b'>') => self.application_keypad = false,
            (b"", b'7') => self.save_dec_cursor(),
            (b"", b'8') => self.restore_dec_cursor(),
            (b"", b'Z') => self.report_decid(),
            (b"#", b'8') => self.screen_alignment_test(),
            (b"", b'c') => self.reset_to_initial_state(),
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        let Some(command) = params
            .first()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return;
        };
        match command {
            "0" => {
                if let Some(label) = params
                    .get(1)
                    .and_then(|bytes| decode_bounded_osc_text(bytes))
                {
                    self.icon_label = Some(label.to_owned());
                    self.title = Some(label.to_owned());
                }
            }
            "1" => {
                if let Some(icon_label) = params
                    .get(1)
                    .and_then(|bytes| decode_bounded_osc_text(bytes))
                {
                    self.icon_label = Some(icon_label.to_owned());
                }
            }
            "2" => {
                if let Some(title) = params
                    .get(1)
                    .and_then(|bytes| decode_bounded_osc_text(bytes))
                {
                    self.title = Some(title.to_owned());
                }
            }
            "52" => {
                if let Some(text) = decode_osc52_clipboard(params) {
                    self.clipboard_text = Some(text);
                }
            }
            "8" => match decode_osc8_hyperlink(params) {
                Osc8HyperlinkAction::Open(uri) => {
                    self.current_hyperlink_id = self.intern_hyperlink(uri);
                }
                Osc8HyperlinkAction::Close => self.current_hyperlink_id = 0,
                Osc8HyperlinkAction::Ignore => {}
            },
            _ => {}
        }
    }
}
