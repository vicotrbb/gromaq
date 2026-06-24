//! Deterministic terminal state engine.

use vte::{Params, Parser, Perform};
use winit::keyboard::{Key, ModifiersState, PhysicalKey};

use crate::cell::{Color, Style};
use crate::dirty::{DirtyRegion, DirtyTracker};
use crate::error::Result;
use crate::grid::Grid;
use crate::input::encode_winit_key_with_terminal_modes;
use crate::mouse::{MouseEvent, MouseReportState};
use crate::scrollback::Scrollback;
use crate::selection::SelectionRange;

mod edit;
mod modes;
mod osc;
mod params;
mod print;
mod reflow;
mod reports;
mod scroll;
mod selection_copy;
mod sgr;
mod snapshot;
mod state;
mod types;
mod view;
mod width;

use osc::{
    Osc8HyperlinkAction, decode_bounded_osc_text, decode_osc8_hyperlink, decode_osc52_clipboard,
};
use params::{default_tab_stops, first_value, first_values};
use snapshot::cell_screenshot_color;
use state::{CharacterSet, Cursor, DcsHandler, DirtyRun, SavedCursorState, SavedScreen};
pub use types::{CursorShape, CursorSnapshot, PerfSnapshot, Screenshot, TerminalConfig};
use width::{map_dec_special_graphics, metadata_id_for_index};

const MAX_OSC_TITLE_BYTES: usize = 4096;
const MAX_OSC52_CLIPBOARD_BYTES: usize = 1_048_576;
const MAX_OSC8_HYPERLINK_BYTES: usize = 4096;
const MAX_METADATA_IDS: usize = 4096;
const MAX_OSC8_HYPERLINKS: usize = MAX_METADATA_IDS;
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
