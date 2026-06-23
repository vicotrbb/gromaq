//! Deterministic terminal state engine.

use base64::{Engine as _, engine::general_purpose};
use unicode_width::UnicodeWidthChar;
use vte::{Params, Parser, Perform};
use winit::keyboard::{Key, ModifiersState, PhysicalKey};

use crate::cell::{Cell, CellSnapshot, Color, Style, UnderlineStyle};
use crate::clipboard::HostClipboard;
use crate::config::validate_terminal_dimensions;
use crate::dirty::{DirtyRegion, DirtyTracker};
use crate::error::{GromaqError, Result};
use crate::grid::{Grid, GridSnapshot};
use crate::input::encode_winit_key_with_terminal_modes;
use crate::mouse::{MouseEvent, MouseReportState};
use crate::scrollback::{Scrollback, ScrollbackSnapshot};
use crate::selection::{SelectionPoint, SelectionRange};

const MAX_SCROLLBACK_LINES: usize = 1_000_000;
const MAX_OSC52_CLIPBOARD_BYTES: usize = 1_048_576;
const MAX_OSC8_HYPERLINK_BYTES: usize = 4096;
const MAX_OSC8_HYPERLINKS: usize = u16::MAX as usize;
const MAX_UNDERLINE_COLORS: usize = u16::MAX as usize;
const MAX_DCS_PAYLOAD_BYTES: usize = 64;

/// Core terminal dimensions and scrollback configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalConfig {
    cols: u16,
    rows: u16,
    pixel_width: u16,
    pixel_height: u16,
    scrollback_limit: usize,
}

impl TerminalConfig {
    /// Build a terminal configuration with a default bounded scrollback.
    pub fn new(cols: u16, rows: u16) -> Result<Self> {
        Self {
            cols,
            rows,
            pixel_width: 0,
            pixel_height: 0,
            scrollback_limit: 10_000,
        }
        .validate()
    }

    /// Set the current native pixel size, when known.
    pub fn with_pixel_size(mut self, pixel_width: u16, pixel_height: u16) -> Result<Self> {
        self.pixel_width = pixel_width;
        self.pixel_height = pixel_height;
        self.validate()
    }

    /// Set the scrollback line limit.
    pub fn with_scrollback_limit(mut self, scrollback_limit: usize) -> Result<Self> {
        self.scrollback_limit = scrollback_limit;
        self.validate()
    }

    /// Number of columns.
    pub fn cols(&self) -> u16 {
        self.cols
    }

    /// Number of rows.
    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// Native pixel width, or zero when unknown.
    pub fn pixel_width(&self) -> u16 {
        self.pixel_width
    }

    /// Native pixel height, or zero when unknown.
    pub fn pixel_height(&self) -> u16 {
        self.pixel_height
    }

    /// Maximum number of scrollback lines.
    pub fn scrollback_limit(&self) -> usize {
        self.scrollback_limit
    }

    fn validate(self) -> Result<Self> {
        validate_terminal_dimensions(self.cols, self.rows)?;
        if self.scrollback_limit > MAX_SCROLLBACK_LINES {
            return Err(GromaqError::InvalidScrollback {
                maximum: MAX_SCROLLBACK_LINES,
                actual: self.scrollback_limit,
            });
        }
        Ok(self)
    }
}

/// Cursor position snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorSnapshot {
    /// Zero-based row.
    pub row: u16,
    /// Zero-based column.
    pub col: u16,
    /// Cursor visibility.
    pub visible: bool,
    /// Cursor shape.
    pub shape: CursorShape,
    /// Whether cursor blinking is requested.
    pub blinking: bool,
}

/// Cursor shape requested by terminal control sequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    /// Block cursor.
    Block,
    /// Underline cursor.
    Underline,
    /// Vertical bar cursor.
    Bar,
}

/// Lightweight performance counters for deterministic tests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PerfSnapshot {
    /// Bytes fed into the parser.
    pub parsed_bytes: u64,
    /// Number of cells dirtied by terminal operations.
    pub dirty_cells: u64,
    /// Number of scroll operations.
    pub scrolls: u64,
    /// Number of successful terminal resize operations.
    pub resizes: u64,
    /// Number of non-empty dirty-region batches drained for rendering.
    pub dirty_region_batches: u64,
}

/// Deterministic in-memory cell screenshot used by the test API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screenshot {
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// RGBA8 pixels.
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
struct Cursor {
    row: u16,
    col: u16,
    visible: bool,
    shape: CursorShape,
    blinking: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharacterSet {
    G0,
    G1,
}

#[derive(Debug, Clone, Copy)]
struct SavedCursorState {
    cursor: Cursor,
    style: Style,
    g0_dec_special_graphics: bool,
    g1_dec_special_graphics: bool,
    active_charset: CharacterSet,
}

#[derive(Debug, Clone)]
struct SavedScreen {
    grid: Grid,
    cursor: Cursor,
    hard_breaks: Vec<bool>,
    tab_stops: Vec<bool>,
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
}

#[derive(Debug, Clone, Copy)]
struct DirtyRun {
    row: u16,
    col_start: u16,
    col_end: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DcsHandler {
    Decrqss,
}

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
        self.tab_stops = default_tab_stops(config.cols);
        self.scroll_top = 0;
        self.scroll_bottom = config.rows - 1;
        self.cursor.row = self.cursor.row.min(config.rows - 1);
        self.cursor.col = self.cursor.col.min(config.cols - 1);
        self.wrap_pending = false;
        self.selection = None;
        self.dirty.mark_viewport(config.rows, config.cols);
        self.config = config;
        self.perf.resizes += 1;
        Ok(())
    }

    /// Return a grid snapshot.
    pub fn dump_grid(&self) -> GridSnapshot {
        GridSnapshot {
            cols: self.config.cols,
            rows: self.config.rows,
            hyperlinks: self.hyperlinks.clone(),
            underline_colors: self.underline_colors.clone(),
            cells: (0..self.config.rows)
                .flat_map(|row| {
                    (0..self.config.cols)
                        .map(move |col| self.snapshot_cell(self.grid.cell(row, col)))
                })
                .collect(),
        }
    }

    fn snapshot_cell(&self, cell: &Cell) -> CellSnapshot {
        CellSnapshot {
            text: cell.text.clone(),
            style: cell.style,
            hyperlink_id: cell.hyperlink_id,
            is_wide_leading: cell.is_wide_leading,
            is_wide_trailing: cell.is_wide_trailing,
        }
    }

    fn trimmed_visible_row_snapshot(&self, row: u16) -> Vec<CellSnapshot> {
        let Some(last_col) = (0..self.config.cols).rev().find(|col| {
            let cell = self.grid.cell(row, *col);
            !cell.text.is_empty() && !cell.is_wide_trailing
        }) else {
            return Vec::new();
        };
        let last_col = if last_col + 1 < self.config.cols
            && self.grid.cell(row, last_col + 1).is_wide_trailing
        {
            last_col + 1
        } else {
            last_col
        };
        (0..=last_col)
            .map(|col| self.snapshot_cell(self.grid.cell(row, col)))
            .collect()
    }

    /// Return a scrollback snapshot.
    pub fn dump_scrollback(&self) -> ScrollbackSnapshot {
        let mut snapshot = self.scrollback.snapshot();
        snapshot.hyperlinks.clone_from(&self.hyperlinks);
        snapshot.underline_colors.clone_from(&self.underline_colors);
        snapshot
    }

    /// Return a cursor snapshot.
    pub fn dump_cursor(&self) -> CursorSnapshot {
        CursorSnapshot {
            row: self.cursor.row,
            col: self.cursor.col,
            visible: self.cursor.visible,
            shape: self.cursor.shape,
            blinking: self.cursor.blinking,
        }
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

    /// Set a visible-grid selection.
    pub fn set_selection(&mut self, selection: SelectionRange) {
        self.selection = Some(selection);
    }

    /// Clear the active selection.
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Copy the active selection as plain text.
    pub fn copy_selection(&self) -> Option<String> {
        let selection = self.selection?;
        Some(self.copy_range(selection))
    }

    /// Copy the active selection into a host clipboard adapter.
    pub fn copy_selection_to_clipboard(
        &self,
        clipboard: &mut impl HostClipboard,
    ) -> Option<String> {
        let text = self.copy_selection()?;
        clipboard.write_text(&text);
        Some(text)
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
        for row in 0..self.config.rows {
            for col in 0..self.config.cols {
                let color =
                    if self.cursor.visible && self.cursor.row == row && self.cursor.col == col {
                        [64, 160, 255, 255]
                    } else {
                        cell_screenshot_color(self.grid.cell(row, col))
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
        let width = UnicodeWidthChar::width(ch).unwrap_or(0).min(2);
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
        let width_u16 = u16::try_from(width).expect("character width is clamped to 2");
        if self.auto_wrap && (self.wrap_pending || self.cursor.col + width_u16 > self.config.cols) {
            self.wrap_pending = false;
            self.carriage_return();
            self.line_feed();
        }
        if self.insert_mode {
            self.insert_blank_chars(width_u16);
        }
        self.clear_stale_wide_neighbors(self.cursor.row, self.cursor.col);
        let cell = self.grid.cell_mut(self.cursor.row, self.cursor.col);
        *cell = Cell {
            text: ch.to_string(),
            style: self.style,
            hyperlink_id: self.current_hyperlink_id,
            is_wide_leading: width == 2,
            is_wide_trailing: false,
        };
        self.mark_print_span(self.cursor.row, self.cursor.col, width_u16);
        self.perf.dirty_cells += 1;
        if width == 2 && self.cursor.col + 1 < self.config.cols {
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
        if self.cursor.col + width_u16 >= self.config.cols {
            self.cursor.col = self.config.cols - 1;
            self.wrap_pending = self.auto_wrap;
        } else {
            self.cursor.col += width_u16;
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

        self.append_to_previous_cluster(ch, col, span_width, span_width);
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
        if !self
            .grid
            .cell(self.cursor.row, col)
            .text
            .ends_with('\u{200d}')
        {
            return false;
        }

        let cell = self.grid.cell_mut(self.cursor.row, col);
        cell.text.push(ch);
        self.mark_print_span(self.cursor.row, col, span_width);
        self.perf.dirty_cells += u64::from(span_width);
        self.last_printable_char = Some(ch);
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

    fn cursor_shape_parameter(&self) -> u16 {
        match (self.cursor.shape, self.cursor.blinking) {
            (CursorShape::Block, true) => 1,
            (CursorShape::Block, false) => 2,
            (CursorShape::Underline, true) => 3,
            (CursorShape::Underline, false) => 4,
            (CursorShape::Bar, true) => 5,
            (CursorShape::Bar, false) => 6,
        }
    }

    fn active_sgr_parameters(&self) -> Vec<String> {
        let mut params = Vec::new();
        if self.style.bold {
            params.push("1".to_owned());
        }
        if self.style.dim {
            params.push("2".to_owned());
        }
        if self.style.italic {
            params.push("3".to_owned());
        }
        if self.style.underline {
            params.push(match self.style.underline_style {
                UnderlineStyle::Single => "4".to_owned(),
                UnderlineStyle::Double => "21".to_owned(),
                UnderlineStyle::Curly => "4:3".to_owned(),
                UnderlineStyle::Dotted => "4:4".to_owned(),
                UnderlineStyle::Dashed => "4:5".to_owned(),
            });
        }
        if self.style.blink {
            params.push("5".to_owned());
        }
        if self.style.inverse {
            params.push("7".to_owned());
        }
        if self.style.hidden {
            params.push("8".to_owned());
        }
        if self.style.strikethrough {
            params.push("9".to_owned());
        }
        if self.style.overline {
            params.push("53".to_owned());
        }
        push_sgr_color_parameters(&mut params, 30, 90, 38, self.style.foreground);
        push_sgr_color_parameters(&mut params, 40, 100, 48, self.style.background);
        if let Some(color) = self.active_underline_color() {
            push_sgr_extended_color_parameter(&mut params, 58, color);
        }

        if params.is_empty() {
            params.push("0".to_owned());
        }
        params
    }

    fn active_underline_color(&self) -> Option<Color> {
        if self.style.underline_color_id == 0 {
            return None;
        }
        self.underline_colors
            .get(usize::from(self.style.underline_color_id - 1))
            .copied()
            .filter(|color| *color != Color::Default)
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

    fn save_dec_cursor(&mut self) {
        self.saved_dec_cursor = Some(SavedCursorState {
            cursor: self.cursor,
            style: self.style,
            g0_dec_special_graphics: self.g0_dec_special_graphics,
            g1_dec_special_graphics: self.g1_dec_special_graphics,
            active_charset: self.active_charset,
        });
    }

    fn restore_dec_cursor(&mut self) {
        if let Some(saved) = self.saved_dec_cursor {
            self.cursor = saved.cursor;
            self.style = saved.style;
            self.g0_dec_special_graphics = saved.g0_dec_special_graphics;
            self.g1_dec_special_graphics = saved.g1_dec_special_graphics;
            self.active_charset = saved.active_charset;
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
                53 => self.style.overline = true,
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
                        match param {
                            38 => self.style.foreground = color,
                            48 => self.style.background = color,
                            58 => {
                                self.style.underline_color_id = self.intern_underline_color(color)
                            }
                            _ => unreachable!("extended color SGR parameter is matched above"),
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn set_private_mode(&mut self, mode: u16, enabled: bool) {
        match mode {
            1 => self.application_cursor_keys = enabled,
            6 => {
                self.origin_mode = enabled;
                self.cursor.row = if enabled { self.scroll_top } else { 0 };
                self.cursor.col = 0;
                self.wrap_pending = false;
            }
            7 => {
                self.auto_wrap = enabled;
                if !enabled {
                    self.wrap_pending = false;
                }
            }
            12 => self.cursor.blinking = enabled,
            25 => self.cursor.visible = enabled,
            66 => self.application_keypad = enabled,
            47 | 1047 if enabled => self.enter_alternate_screen(),
            47 | 1047 => self.leave_alternate_screen(),
            1048 if enabled => self.save_dec_cursor(),
            1048 => self.restore_dec_cursor(),
            1049 if enabled => {
                if self.saved_primary.is_none() {
                    self.save_dec_cursor();
                }
                self.enter_alternate_screen();
            }
            1049 => {
                let was_in_alternate_screen = self.saved_primary.is_some();
                self.leave_alternate_screen();
                if was_in_alternate_screen {
                    self.restore_dec_cursor();
                }
            }
            1000 => self.mouse.set_button_reporting(enabled),
            1002 => self.mouse.set_button_motion_reporting(enabled),
            1003 => self.mouse.set_any_motion_reporting(enabled),
            1004 => self.focus_event_reporting = enabled,
            1006 => self.mouse.set_sgr_protocol(enabled),
            2004 => self.bracketed_paste = enabled,
            _ => {}
        }
    }

    fn private_mode_state(&self, mode: u16) -> Option<bool> {
        match mode {
            1 => Some(self.application_cursor_keys),
            6 => Some(self.origin_mode),
            7 => Some(self.auto_wrap),
            12 => Some(self.cursor.blinking),
            25 => Some(self.cursor.visible),
            66 => Some(self.application_keypad),
            1000 => Some(self.mouse.button_reporting_enabled()),
            1002 => Some(self.mouse.button_motion_reporting_enabled()),
            1003 => Some(self.mouse.any_motion_reporting_enabled()),
            1004 => Some(self.focus_event_reporting),
            1006 => Some(self.mouse.sgr_protocol_enabled()),
            2004 => Some(self.bracketed_paste),
            _ => None,
        }
    }

    fn save_private_modes(&mut self, modes: &[u16]) {
        for mode in modes {
            let Some(enabled) = self.private_mode_state(*mode) else {
                continue;
            };
            if let Some((_, saved)) = self
                .saved_private_modes
                .iter_mut()
                .find(|(saved_mode, _)| saved_mode == mode)
            {
                *saved = enabled;
            } else {
                self.saved_private_modes.push((*mode, enabled));
            }
        }
    }

    fn restore_private_modes(&mut self, modes: &[u16]) {
        let restores: Vec<(u16, bool)> = modes
            .iter()
            .filter_map(|mode| {
                self.saved_private_modes
                    .iter()
                    .find(|(saved_mode, _)| saved_mode == mode)
                    .copied()
            })
            .collect();
        for (mode, enabled) in restores {
            self.set_private_mode(mode, enabled);
        }
    }

    fn set_mode(&mut self, mode: u16, enabled: bool) {
        match mode {
            4 => self.insert_mode = enabled,
            20 => self.linefeed_newline_mode = enabled,
            _ => {}
        }
    }

    fn mode_state(&self, mode: u16) -> Option<bool> {
        match mode {
            4 => Some(self.insert_mode),
            20 => Some(self.linefeed_newline_mode),
            _ => None,
        }
    }

    fn report_mode_state(&mut self, private: bool, mode: u16) {
        let state = if private {
            self.private_mode_state(mode)
        } else {
            self.mode_state(mode)
        };
        let value = match state {
            Some(true) => 1,
            Some(false) => 2,
            None => 0,
        };

        if private {
            self.pending_response_bytes
                .extend_from_slice(format!("\x1b[?{};{}$y", mode, value).as_bytes());
        } else {
            self.pending_response_bytes
                .extend_from_slice(format!("\x1b[{};{}$y", mode, value).as_bytes());
        }
    }

    fn report_device_status(&mut self, mode: u16) {
        match mode {
            5 => self.pending_response_bytes.extend_from_slice(b"\x1b[0n"),
            6 => self.pending_response_bytes.extend_from_slice(
                format!("\x1b[{};{}R", self.cursor.row + 1, self.cursor.col + 1).as_bytes(),
            ),
            _ => {}
        }
    }

    fn report_private_device_status(&mut self, mode: u16) {
        match mode {
            6 => self.pending_response_bytes.extend_from_slice(
                format!("\x1b[?{};{}R", self.cursor.row + 1, self.cursor.col + 1).as_bytes(),
            ),
            15 => self.pending_response_bytes.extend_from_slice(b"\x1b[?11n"),
            25 => self.pending_response_bytes.extend_from_slice(b"\x1b[?20n"),
            26 => self
                .pending_response_bytes
                .extend_from_slice(b"\x1b[?27;1;0;0n"),
            53 => self.pending_response_bytes.extend_from_slice(b"\x1b[?50n"),
            _ => {}
        }
    }

    fn report_terminal_parameters(&mut self, mode: u16) {
        match mode {
            0 | 1 => self
                .pending_response_bytes
                .extend_from_slice(format!("\x1b[{};1;1;128;128;1;0x", mode + 2).as_bytes()),
            _ => {}
        }
    }

    fn report_decrqss(&mut self, request: &[u8]) {
        match request {
            b"m" => self.pending_response_bytes.extend_from_slice(
                format!("\x1bP1$r{}m\x1b\\", self.active_sgr_parameters().join(";")).as_bytes(),
            ),
            b"r" => self.pending_response_bytes.extend_from_slice(
                format!(
                    "\x1bP1$r{};{}r\x1b\\",
                    self.scroll_top + 1,
                    self.scroll_bottom + 1
                )
                .as_bytes(),
            ),
            b" q" => self.pending_response_bytes.extend_from_slice(
                format!("\x1bP1$r{} q\x1b\\", self.cursor_shape_parameter()).as_bytes(),
            ),
            _ => self
                .pending_response_bytes
                .extend_from_slice(b"\x1bP0$r\x1b\\"),
        }
    }

    fn report_window_manipulation(&mut self, mode: u16) {
        match mode {
            11 => self.pending_response_bytes.extend_from_slice(b"\x1b[1t"),
            13 => self
                .pending_response_bytes
                .extend_from_slice(b"\x1b[3;0;0t"),
            14 => self.pending_response_bytes.extend_from_slice(
                format!(
                    "\x1b[4;{};{}t",
                    self.config.pixel_height, self.config.pixel_width
                )
                .as_bytes(),
            ),
            18 => self.pending_response_bytes.extend_from_slice(
                format!("\x1b[8;{};{}t", self.config.rows, self.config.cols).as_bytes(),
            ),
            19 => self.pending_response_bytes.extend_from_slice(
                format!("\x1b[9;{};{}t", self.config.rows, self.config.cols).as_bytes(),
            ),
            20 => {
                self.pending_response_bytes.extend_from_slice(b"\x1b]L");
                if let Some(icon_label) = self.icon_label.as_ref().or(self.title.as_ref()) {
                    self.pending_response_bytes
                        .extend_from_slice(icon_label.as_bytes());
                }
                self.pending_response_bytes.extend_from_slice(b"\x1b\\");
            }
            21 => {
                self.pending_response_bytes.extend_from_slice(b"\x1b]l");
                if let Some(title) = &self.title {
                    self.pending_response_bytes
                        .extend_from_slice(title.as_bytes());
                }
                self.pending_response_bytes.extend_from_slice(b"\x1b\\");
            }
            _ => {}
        }
    }

    fn report_primary_device_attributes(&mut self, mode: u16) {
        if mode == 0 {
            self.pending_response_bytes.extend_from_slice(b"\x1b[?1;2c");
        }
    }

    fn report_secondary_device_attributes(&mut self, mode: u16) {
        if mode == 0 {
            self.pending_response_bytes
                .extend_from_slice(b"\x1b[>0;1;0c");
        }
    }

    #[cold]
    #[inline(never)]
    fn report_decid(&mut self) {
        self.report_primary_device_attributes(0);
    }

    fn intern_hyperlink(&mut self, uri: String) -> u16 {
        if let Some(index) = self.hyperlinks.iter().position(|existing| existing == &uri) {
            return u16::try_from(index + 1).expect("hyperlink table length is capped");
        }
        if self.hyperlinks.len() == MAX_OSC8_HYPERLINKS {
            return 0;
        }
        self.hyperlinks.push(uri);
        u16::try_from(self.hyperlinks.len()).expect("hyperlink table length is capped")
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
            return u16::try_from(index + 1).expect("underline color table length is capped");
        }
        if self.underline_colors.len() == MAX_UNDERLINE_COLORS {
            return 0;
        }
        self.underline_colors.push(color);
        u16::try_from(self.underline_colors.len()).expect("underline color table length is capped")
    }

    fn enter_alternate_screen(&mut self) {
        if self.saved_primary.is_some() {
            return;
        }
        self.flush_dirty_run();
        self.saved_primary = Some(SavedScreen {
            grid: self.grid.clone(),
            cursor: self.cursor,
            hard_breaks: self.hard_breaks.clone(),
            tab_stops: self.tab_stops.clone(),
            wrap_pending: self.wrap_pending,
            auto_wrap: self.auto_wrap,
            origin_mode: self.origin_mode,
            application_cursor_keys: self.application_cursor_keys,
            application_keypad: self.application_keypad,
            focus_event_reporting: self.focus_event_reporting,
            insert_mode: self.insert_mode,
            linefeed_newline_mode: self.linefeed_newline_mode,
            g0_dec_special_graphics: self.g0_dec_special_graphics,
            g1_dec_special_graphics: self.g1_dec_special_graphics,
            active_charset: self.active_charset,
            scroll_top: self.scroll_top,
            scroll_bottom: self.scroll_bottom,
        });
        self.grid = Grid::new(self.config.cols, self.config.rows);
        self.hard_breaks = vec![false; usize::from(self.config.rows)];
        self.tab_stops = default_tab_stops(self.config.cols);
        self.scroll_top = 0;
        self.scroll_bottom = self.config.rows - 1;
        self.cursor.row = 0;
        self.cursor.col = 0;
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
        self.selection = None;
        self.dirty.mark_viewport(self.config.rows, self.config.cols);
    }

    fn leave_alternate_screen(&mut self) {
        if let Some(saved) = self.saved_primary.take() {
            self.flush_dirty_run();
            self.grid = saved.grid;
            self.cursor = saved.cursor;
            self.hard_breaks = saved.hard_breaks;
            self.tab_stops = saved.tab_stops;
            self.wrap_pending = saved.wrap_pending;
            self.auto_wrap = saved.auto_wrap;
            self.origin_mode = saved.origin_mode;
            self.application_cursor_keys = saved.application_cursor_keys;
            self.application_keypad = saved.application_keypad;
            self.focus_event_reporting = saved.focus_event_reporting;
            self.insert_mode = saved.insert_mode;
            self.linefeed_newline_mode = saved.linefeed_newline_mode;
            self.g0_dec_special_graphics = saved.g0_dec_special_graphics;
            self.g1_dec_special_graphics = saved.g1_dec_special_graphics;
            self.active_charset = saved.active_charset;
            self.scroll_top = saved.scroll_top;
            self.scroll_bottom = saved.scroll_bottom;
            self.selection = None;
            self.dirty.mark_viewport(self.config.rows, self.config.cols);
        }
    }

    fn copy_range(&self, selection: SelectionRange) -> String {
        let selection = self.clamp_selection_to_viewport(selection);
        let mut output = String::new();
        for row in selection.start.row..=selection.end.row {
            let start_col = if row == selection.start.row {
                selection.start.col
            } else {
                0
            };
            let end_col = if row == selection.end.row {
                selection.end.col
            } else {
                self.config.cols - 1
            };
            output.push_str(&self.copy_row_range(row, start_col, end_col));
            if row < selection.end.row && self.copy_boundary_needs_newline(row, end_col) {
                output.push('\n');
            }
        }
        output
    }

    fn clamp_selection_to_viewport(&self, selection: SelectionRange) -> SelectionRange {
        let start = self.clamp_selection_point(selection.start);
        let end = self.clamp_selection_point(selection.end);
        if start <= end {
            SelectionRange { start, end }
        } else {
            SelectionRange {
                start: end,
                end: start,
            }
        }
    }

    fn clamp_selection_point(&self, point: SelectionPoint) -> SelectionPoint {
        SelectionPoint {
            row: point.row.min(self.config.rows - 1),
            col: point.col.min(self.config.cols - 1),
        }
    }

    fn copy_row_range(&self, row: u16, start_col: u16, end_col: u16) -> String {
        let start_col = self.copy_start_col(row, start_col);
        let Some(end_col) = self
            .last_visible_col_in_row(row)
            .map(|last_col| end_col.min(last_col))
        else {
            return String::new();
        };

        if end_col < start_col {
            return String::new();
        }

        let mut output = String::new();
        for col in start_col..=end_col {
            let cell = self.grid.cell(row, col);
            if cell.is_wide_trailing {
                continue;
            }
            if cell.text.is_empty() {
                output.push(' ');
            } else {
                output.push_str(&cell.text);
            }
        }
        output
    }

    fn copy_start_col(&self, row: u16, start_col: u16) -> u16 {
        if start_col > 0 && self.grid.cell(row, start_col).is_wide_trailing {
            start_col - 1
        } else {
            start_col
        }
    }

    fn last_visible_col_in_row(&self, row: u16) -> Option<u16> {
        (0..self.config.cols).rev().find(|col| {
            let cell = self.grid.cell(row, *col);
            !cell.text.is_empty() && !cell.is_wide_trailing
        })
    }

    fn copy_boundary_needs_newline(&self, row: u16, end_col: u16) -> bool {
        self.hard_breaks
            .get(usize::from(row))
            .copied()
            .unwrap_or(false)
            || end_col < self.config.cols - 1
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
        let lines = self.visible_logical_lines();
        let mut grid = Grid::new(cols, rows);
        let mut hard_breaks = vec![false; usize::from(rows)];
        let mut row = 0;
        let mut col = 0;

        for (line_index, line) in lines.iter().enumerate() {
            for unit in &line.cells {
                if col + unit.width > cols {
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
                    is_wide_leading: unit.width == 2,
                    is_wide_trailing: false,
                };
                if unit.width == 2 && col + 1 < cols {
                    *grid.cell_mut(row, col + 1) = Cell {
                        text: String::new(),
                        style: unit.style,
                        hyperlink_id: unit.hyperlink_id,
                        is_wide_leading: false,
                        is_wide_trailing: true,
                    };
                }
                col += unit.width;
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

    fn visible_logical_lines(&self) -> Vec<LogicalLine> {
        let mut lines = Vec::new();
        let mut current = Vec::new();

        for row in 0..self.config.rows {
            let cells = self.visible_row_units(row);
            let is_hard_break = self
                .hard_breaks
                .get(usize::from(row))
                .copied()
                .unwrap_or(false);
            let is_full_soft_row = !is_hard_break
                && cells
                    .iter()
                    .map(|cell| usize::from(cell.width))
                    .sum::<usize>()
                    >= usize::from(self.config.cols);

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

    fn visible_row_units(&self, row: u16) -> Vec<ReflowCell> {
        let Some(last_col) = self.last_visible_col(row) else {
            return Vec::new();
        };

        let mut units = Vec::new();
        for col in 0..=last_col {
            let cell = self.grid.cell(row, col);
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
            let width = if cell.is_wide_leading {
                2
            } else {
                u16::try_from(visible_width(&cell.text).clamp(1, 2))
                    .expect("visible width is clamped")
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

    fn last_visible_col(&self, row: u16) -> Option<u16> {
        (0..self.config.cols).rev().find(|col| {
            let cell = self.grid.cell(row, *col);
            !cell.text.is_empty() && !cell.is_wide_trailing
        })
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

fn visible_width(text: &str) -> usize {
    text.chars()
        .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0).min(2))
        .sum()
}

fn is_emoji_modifier(ch: char) -> bool {
    matches!(ch, '\u{1f3fb}'..='\u{1f3ff}')
}

fn is_emoji_modifier_base_candidate(ch: char) -> bool {
    matches!(
        ch,
        '\u{2600}'..='\u{27bf}' | '\u{1f000}'..='\u{1faff}'
    )
}

fn is_emoji_presentation_base_candidate(ch: char) -> bool {
    is_emoji_modifier_base_candidate(ch) || matches!(ch, '\u{00a9}' | '\u{00ae}' | '\u{2122}')
}

fn is_variation_selector_16(ch: char) -> bool {
    ch == '\u{fe0f}'
}

fn is_combining_enclosing_keycap(ch: char) -> bool {
    ch == '\u{20e3}'
}

fn is_keycap_base_sequence(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(base) = chars.next() else {
        return false;
    };
    if !matches!(base, '#' | '*' | '0'..='9') {
        return false;
    }
    matches!(chars.next(), None | Some('\u{fe0f}')) && chars.next().is_none()
}

fn is_regional_indicator(ch: char) -> bool {
    matches!(ch, '\u{1f1e6}'..='\u{1f1ff}')
}

fn cell_screenshot_color(cell: &Cell) -> [u8; 4] {
    if cell.is_wide_trailing {
        return [255, 255, 255, 255];
    }
    if cell.text.is_empty() {
        return color_to_rgba(cell.style.background, [0, 0, 0, 255]);
    }
    color_to_rgba(cell.style.foreground, [255, 255, 255, 255])
}

fn color_to_rgba(color: Color, default: [u8; 4]) -> [u8; 4] {
    match color {
        Color::Default => default,
        Color::Ansi(index) => ansi_color_to_rgba(index),
        Color::Indexed(index) => indexed_color_to_rgba(index),
        Color::Rgb(red, green, blue) => [red, green, blue, 255],
    }
}

fn ansi_color_to_rgba(index: u8) -> [u8; 4] {
    const ANSI: [[u8; 4]; 16] = [
        [0, 0, 0, 255],
        [205, 49, 49, 255],
        [13, 188, 121, 255],
        [229, 229, 16, 255],
        [36, 114, 200, 255],
        [188, 63, 188, 255],
        [17, 168, 205, 255],
        [229, 229, 229, 255],
        [102, 102, 102, 255],
        [241, 76, 76, 255],
        [35, 209, 139, 255],
        [245, 245, 67, 255],
        [59, 142, 234, 255],
        [214, 112, 214, 255],
        [41, 184, 219, 255],
        [255, 255, 255, 255],
    ];
    ANSI[usize::from(index.min(15))]
}

fn indexed_color_to_rgba(index: u8) -> [u8; 4] {
    if index < 16 {
        return ansi_color_to_rgba(index);
    }
    if index < 232 {
        let offset = index - 16;
        let red = color_cube_component(offset / 36);
        let green = color_cube_component((offset / 6) % 6);
        let blue = color_cube_component(offset % 6);
        return [red, green, blue, 255];
    }
    let gray = 8 + (index - 232) * 10;
    [gray, gray, gray, 255]
}

fn color_cube_component(value: u8) -> u8 {
    if value == 0 { 0 } else { 55 + value * 40 }
}

#[cold]
#[inline(never)]
fn map_dec_special_graphics(ch: char) -> char {
    match ch {
        '`' => '◆',
        'a' => '▒',
        'f' => '°',
        'g' => '±',
        'h' => '␤',
        'i' => '␋',
        'j' => '┘',
        'k' => '┐',
        'l' => '┌',
        'm' => '└',
        'n' => '┼',
        'o' => '⎺',
        'p' => '⎻',
        'q' => '─',
        'r' => '⎼',
        's' => '⎽',
        't' => '├',
        'u' => '┤',
        'v' => '┴',
        'w' => '┬',
        'x' => '│',
        'y' => '≤',
        'z' => '≥',
        '{' => 'π',
        '|' => '≠',
        '}' => '£',
        '~' => '·',
        _ => ch,
    }
}

impl Perform for Terminal {
    fn hook(&mut self, _params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        self.dcs_payload.clear();
        self.dcs_handler = if !ignore && intermediates == b"$" && action == 'q' {
            Some(DcsHandler::Decrqss)
        } else {
            None
        };
    }

    fn put(&mut self, byte: u8) {
        if self.dcs_handler.is_some() && self.dcs_payload.len() < MAX_DCS_PAYLOAD_BYTES {
            self.dcs_payload.push(byte);
        } else {
            self.dcs_handler = None;
            self.dcs_payload.clear();
        }
    }

    fn unhook(&mut self) {
        let Some(DcsHandler::Decrqss) = self.dcs_handler.take() else {
            self.dcs_payload.clear();
            return;
        };
        let request = std::mem::take(&mut self.dcs_payload);
        self.report_decrqss(&request);
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
        let values = first_values(params);
        let first = values.first().copied().unwrap_or(0);
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
                let row = values.first().copied().unwrap_or(1);
                let col = values.get(1).copied().unwrap_or(1);
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
            'r' if intermediates == b"?" => self.restore_private_modes(&values),
            'r' => {
                let top = values.first().copied().unwrap_or(1);
                let bottom = values.get(1).copied().unwrap_or(self.config.rows);
                self.set_scroll_region(top, bottom);
            }
            's' if intermediates == b"?" => self.save_private_modes(&values),
            's' => self.save_cursor(),
            't' if intermediates.is_empty() => self.report_window_manipulation(first),
            'u' => self.restore_cursor(),
            'x' if intermediates.is_empty() => self.report_terminal_parameters(first),
            'h' if intermediates.is_empty() => {
                for mode in values {
                    self.set_mode(mode, true);
                }
            }
            'l' if intermediates.is_empty() => {
                for mode in values {
                    self.set_mode(mode, false);
                }
            }
            'h' if intermediates == b"?" => {
                for mode in values {
                    self.set_private_mode(mode, true);
                }
            }
            'l' if intermediates == b"?" => {
                for mode in values {
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
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                {
                    self.icon_label = Some(label.to_owned());
                    self.title = Some(label.to_owned());
                }
            }
            "1" => {
                if let Some(icon_label) = params
                    .get(1)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                {
                    self.icon_label = Some(icon_label.to_owned());
                }
            }
            "2" => {
                if let Some(title) = params
                    .get(1)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
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

fn first_values(params: &Params) -> Vec<u16> {
    params
        .iter()
        .map(|param| param.first().copied().unwrap_or(0))
        .collect()
}

fn default_tab_stops(cols: u16) -> Vec<bool> {
    let mut tab_stops = vec![false; usize::from(cols)];
    for col in (8..usize::from(cols)).step_by(8) {
        tab_stops[col] = true;
    }
    tab_stops
}

fn push_sgr_color_parameters(
    params: &mut Vec<String>,
    normal_base: u16,
    bright_base: u16,
    extended_prefix: u16,
    color: Color,
) {
    match color {
        Color::Default => {}
        Color::Ansi(index) if index < 8 => {
            params.push((normal_base + u16::from(index)).to_string());
        }
        Color::Ansi(index) if index < 16 => {
            params.push((bright_base + u16::from(index - 8)).to_string());
        }
        Color::Ansi(index) | Color::Indexed(index) => {
            params.push(format!("{extended_prefix}:5:{index}"));
        }
        Color::Rgb(red, green, blue) => {
            params.push(format!("{extended_prefix}:2:{red}:{green}:{blue}"));
        }
    }
}

fn push_sgr_extended_color_parameter(params: &mut Vec<String>, prefix: u16, color: Color) {
    match color {
        Color::Default => {}
        Color::Ansi(index) | Color::Indexed(index) => {
            params.push(format!("{prefix}:5:{index}"));
        }
        Color::Rgb(red, green, blue) => {
            params.push(format!("{prefix}:2:{red}:{green}:{blue}"));
        }
    }
}

fn parse_extended_color<I>(iter: &mut std::iter::Peekable<I>) -> Option<Color>
where
    I: Iterator<Item = u16>,
{
    match iter.next()? {
        5 => {
            let index = u8::try_from(iter.next()?).ok()?;
            Some(Color::Indexed(index))
        }
        2 => {
            let r = u8::try_from(iter.next()?).ok()?;
            let g = u8::try_from(iter.next()?).ok()?;
            let b = u8::try_from(iter.next()?).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

fn is_invalid_grouped_extended_color_param(param: &[u16]) -> bool {
    match param {
        [38 | 48 | 58] => false,
        [38 | 48 | 58, 5, index] => u8::try_from(*index).is_err(),
        [38 | 48 | 58, 2, red, green, blue] => {
            u8::try_from(*red).is_err()
                || u8::try_from(*green).is_err()
                || u8::try_from(*blue).is_err()
        }
        [38 | 48 | 58, ..] => true,
        _ => false,
    }
}

fn apply_grouped_sgr_param(style: &mut Style, param: &[u16]) -> bool {
    match param {
        [4, underline_style] => {
            match underline_style {
                0 => {
                    style.underline = false;
                    style.underline_style = UnderlineStyle::Single;
                }
                1 => {
                    style.underline = true;
                    style.underline_style = UnderlineStyle::Single;
                }
                2 => {
                    style.underline = true;
                    style.underline_style = UnderlineStyle::Double;
                }
                3 => {
                    style.underline = true;
                    style.underline_style = UnderlineStyle::Curly;
                }
                4 => {
                    style.underline = true;
                    style.underline_style = UnderlineStyle::Dotted;
                }
                5 => {
                    style.underline = true;
                    style.underline_style = UnderlineStyle::Dashed;
                }
                _ => {}
            }
            true
        }
        _ => false,
    }
}

fn decode_osc52_clipboard(params: &[&[u8]]) -> Option<String> {
    let selector = params
        .get(1)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())?;
    if !selector.is_empty() && !selector.chars().any(|ch| ch == 'c') {
        return None;
    }
    let payload = params
        .get(2)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())?;
    if payload == "?" {
        return None;
    }
    let max_encoded_len = MAX_OSC52_CLIPBOARD_BYTES.div_ceil(3) * 4;
    if payload.len() > max_encoded_len {
        return None;
    }
    let decoded = general_purpose::STANDARD
        .decode(payload)
        .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(payload))
        .ok()?;
    if decoded.len() > MAX_OSC52_CLIPBOARD_BYTES {
        return None;
    }
    String::from_utf8(decoded).ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Osc8HyperlinkAction {
    Open(String),
    Close,
    Ignore,
}

fn decode_osc8_hyperlink(params: &[&[u8]]) -> Osc8HyperlinkAction {
    let Some(uri) = params.get(2) else {
        return Osc8HyperlinkAction::Ignore;
    };
    if uri.is_empty() {
        return Osc8HyperlinkAction::Close;
    }
    if uri.len() > MAX_OSC8_HYPERLINK_BYTES {
        return Osc8HyperlinkAction::Ignore;
    }
    let Ok(uri) = std::str::from_utf8(uri) else {
        return Osc8HyperlinkAction::Ignore;
    };
    Osc8HyperlinkAction::Open(uri.to_owned())
}
