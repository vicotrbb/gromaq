//! Structured deterministic test API.

use crate::error::Result;
use crate::grid::GridSnapshot;
use crate::input::{TestKey, encode_keys};
use crate::scrollback::ScrollbackSnapshot;
use crate::terminal::{CursorSnapshot, PerfSnapshot, Screenshot, Terminal};

/// Required structured terminal test API for debug and validation builds.
pub trait TerminalTestApi {
    /// Send structured keys to the terminal as input bytes.
    fn send_keys(&mut self, keys: &[TestKey]) -> Vec<u8>;
    /// Paste text into the terminal parser.
    fn paste_text(&mut self, text: &str) -> Result<()>;
    /// Resize the terminal grid.
    fn resize(&mut self, cols: u16, rows: u16) -> Result<()>;
    /// Dump the visible grid state.
    fn dump_grid(&self) -> GridSnapshot;
    /// Dump scrollback state.
    fn dump_scrollback(&self) -> ScrollbackSnapshot;
    /// Dump cursor state.
    fn dump_cursor(&self) -> CursorSnapshot;
    /// Dump performance counters.
    fn dump_perf_metrics(&self) -> PerfSnapshot;
    /// Capture a screenshot.
    fn screenshot(&self) -> Screenshot;
}

impl TerminalTestApi for Terminal {
    fn send_keys(&mut self, keys: &[TestKey]) -> Vec<u8> {
        encode_keys(keys)
    }

    fn paste_text(&mut self, text: &str) -> Result<()> {
        self.write_str(text)
    }

    fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        Terminal::resize(self, cols, rows)
    }

    fn dump_grid(&self) -> GridSnapshot {
        Terminal::dump_grid(self)
    }

    fn dump_scrollback(&self) -> ScrollbackSnapshot {
        Terminal::dump_scrollback(self)
    }

    fn dump_cursor(&self) -> CursorSnapshot {
        Terminal::dump_cursor(self)
    }

    fn dump_perf_metrics(&self) -> PerfSnapshot {
        Terminal::dump_perf_metrics(self)
    }

    fn screenshot(&self) -> Screenshot {
        Terminal::screenshot(self)
    }
}
