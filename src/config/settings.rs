//! Structured non-theme configuration sections and validation helpers.

use serde::{Deserialize, Serialize};

use crate::error::{GromaqError, Result};

/// Maximum supported visible terminal grid cells.
pub const MAX_TERMINAL_CELLS: u64 = 1_000_000;
/// Minimum renderable configured font size in pixels.
pub const MIN_FONT_SIZE_PX: f32 = 6.0;
/// Maximum renderable configured font size in pixels.
pub const MAX_FONT_SIZE_PX: f32 = 512.0;
/// Minimum useful terminal cell width in pixels.
pub const MIN_CELL_WIDTH_PX: f32 = 4.0;
/// Maximum useful terminal cell width in pixels.
pub const MAX_CELL_WIDTH_PX: f32 = 512.0;
/// Minimum renderable configured line height in pixels.
pub const MIN_LINE_HEIGHT_PX: f32 = 6.0;
/// Maximum renderable configured line height in pixels.
pub const MAX_LINE_HEIGHT_PX: f32 = 1024.0;
/// Maximum supported target refresh rate for deterministic frame pacing.
pub const MAX_TARGET_FPS: u32 = 1_000;
/// Built-in automatic monospace font stack.
pub const DEFAULT_FONT_FAMILY: &str = "monospace";

/// Terminal section of the configuration file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
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

/// Shell section of the configuration file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellSettings {
    /// Optional shell program path or name. Defaults to the user's system shell.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub program: Option<String>,
    /// Optional shell arguments.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Optional shell working directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// Font section of the configuration file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FontSettings {
    /// Font family name.
    pub family: String,
    /// Font size in pixels.
    pub size_px: f32,
    /// Optional terminal column width in pixels. Defaults to a compact monospace ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell_width_px: Option<f32>,
    /// Terminal row height in pixels.
    pub line_height_px: f32,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            family: DEFAULT_FONT_FAMILY.to_owned(),
            size_px: 34.0,
            cell_width_px: None,
            line_height_px: 47.0,
        }
    }
}

impl FontSettings {
    /// Deterministic renderer font size used for glyph cache keys and render planning.
    pub fn renderer_font_size_px(&self) -> u16 {
        self.size_px.round() as u16
    }

    /// Deterministic renderer cell width used for terminal column geometry.
    pub fn renderer_cell_width_px(&self) -> u16 {
        self.cell_width_px
            .unwrap_or(self.size_px * 0.56)
            .round()
            .max(1.0) as u16
    }

    /// Deterministic renderer row height used for quad planning.
    pub fn renderer_line_height_px(&self) -> u16 {
        self.line_height_px.round() as u16
    }
}

/// Performance section of the configuration file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
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

pub(super) fn validate_shell_settings(shell: &ShellSettings) -> Result<()> {
    if shell
        .program
        .as_ref()
        .is_some_and(|program| program.trim().is_empty())
    {
        return Err(GromaqError::InvalidShellProgram);
    }
    for (index, arg) in shell.args.iter().enumerate() {
        if arg.is_empty() {
            return Err(GromaqError::InvalidShellArgument { index });
        }
    }
    if shell.cwd.as_ref().is_some_and(|cwd| cwd.trim().is_empty()) {
        return Err(GromaqError::InvalidShellCwd);
    }
    Ok(())
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
