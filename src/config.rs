//! User configuration model and validation.

use serde::{Deserialize, Serialize};

use crate::error::{GromaqError, Result};

/// Maximum supported visible terminal grid cells.
pub const MAX_TERMINAL_CELLS: u64 = 1_000_000;

/// Top-level Gromaq configuration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GromaqConfig {
    /// Terminal dimensions and history.
    pub terminal: TerminalSettings,
    /// Font configuration.
    pub font: FontSettings,
    /// Performance-related targets.
    pub performance: PerformanceSettings,
}

impl GromaqConfig {
    /// Validate configuration values.
    pub fn validate(&self) -> Result<()> {
        validate_terminal_dimensions(self.terminal.cols, self.terminal.rows)?;
        if self.terminal.scrollback_lines > 1_000_000 {
            return Err(GromaqError::InvalidScrollback {
                maximum: 1_000_000,
                actual: self.terminal.scrollback_lines,
            });
        }
        if self.font.size_px < 6.0 {
            return Err(GromaqError::InvalidFontSize {
                minimum: 6.0,
                actual: self.font.size_px,
            });
        }
        if self.performance.target_fps == 0 {
            return Err(GromaqError::InvalidTargetFps {
                minimum: 1,
                actual: self.performance.target_fps,
            });
        }
        Ok(())
    }
}

pub(crate) fn validate_terminal_dimensions(cols: u16, rows: u16) -> Result<()> {
    if cols == 0 {
        return Err(GromaqError::InvalidDimension {
            field: "columns",
            minimum: 1,
            actual: u32::from(cols),
        });
    }
    if rows == 0 {
        return Err(GromaqError::InvalidDimension {
            field: "rows",
            minimum: 1,
            actual: u32::from(rows),
        });
    }
    let cells = u64::from(cols) * u64::from(rows);
    if cells > MAX_TERMINAL_CELLS {
        return Err(GromaqError::InvalidGridArea {
            maximum: MAX_TERMINAL_CELLS,
            actual: cells,
        });
    }
    Ok(())
}

/// Terminal section of the configuration file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSettings {
    /// Startup columns.
    pub cols: u16,
    /// Startup rows.
    pub rows: u16,
    /// Scrollback line limit.
    pub scrollback_lines: usize,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            cols: 120,
            rows: 36,
            scrollback_lines: 10_000,
        }
    }
}

/// Font section of the configuration file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontSettings {
    /// Font family name.
    pub family: String,
    /// Font size in pixels.
    pub size_px: f32,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            family: "monospace".to_owned(),
            size_px: 14.0,
        }
    }
}

/// Performance section of the configuration file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerformanceSettings {
    /// Desired maximum refresh rate.
    pub target_fps: u32,
    /// Whether dirty-region rendering is required.
    pub dirty_region_rendering: bool,
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            target_fps: 144,
            dirty_region_rendering: true,
        }
    }
}
