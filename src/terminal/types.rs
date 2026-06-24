//! Public terminal configuration and snapshot types.

use crate::config::validate_terminal_dimensions;
use crate::error::{GromaqError, Result};

const MAX_SCROLLBACK_LINES: usize = 1_000_000;

/// Core terminal dimensions and scrollback configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalConfig {
    pub(super) cols: u16,
    pub(super) rows: u16,
    pub(super) pixel_width: u16,
    pub(super) pixel_height: u16,
    pub(super) scrollback_limit: usize,
    pub(super) cursor_shape: CursorShape,
    pub(super) cursor_blinking: bool,
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
            cursor_shape: CursorShape::Block,
            cursor_blinking: true,
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

    /// Set the default cursor shape before escape sequences override it.
    pub fn with_cursor_shape(mut self, cursor_shape: CursorShape) -> Result<Self> {
        self.cursor_shape = cursor_shape;
        self.validate()
    }

    /// Set whether the default cursor requests blinking.
    pub fn with_cursor_blinking(mut self, cursor_blinking: bool) -> Result<Self> {
        self.cursor_blinking = cursor_blinking;
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

    /// Default cursor shape before escape sequences override it.
    pub fn cursor_shape(&self) -> CursorShape {
        self.cursor_shape
    }

    /// Whether the default cursor requests blinking.
    pub fn cursor_blinking(&self) -> bool {
        self.cursor_blinking
    }

    pub(super) fn validate(self) -> Result<Self> {
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
