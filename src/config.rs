//! User configuration model and validation.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{GromaqError, Result};

/// Maximum supported visible terminal grid cells.
pub const MAX_TERMINAL_CELLS: u64 = 1_000_000;
/// Minimum renderable configured font size in pixels.
pub const MIN_FONT_SIZE_PX: f32 = 6.0;
/// Maximum renderable configured font size in pixels.
pub const MAX_FONT_SIZE_PX: f32 = 512.0;

/// Top-level Gromaq configuration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GromaqConfig {
    /// Terminal dimensions and history.
    pub terminal: TerminalSettings,
    /// Shell command configuration.
    pub shell: ShellSettings,
    /// Font configuration.
    pub font: FontSettings,
    /// Performance-related targets.
    pub performance: PerformanceSettings,
}

impl GromaqConfig {
    /// Load, parse, and validate a TOML configuration file from disk.
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path).map_err(|error| GromaqError::ConfigRead {
            path: path.display().to_string(),
            message: error.to_string(),
        })?;
        Self::from_toml_str(&contents)
    }

    /// Parse and validate TOML configuration text.
    pub fn from_toml_str(contents: &str) -> Result<Self> {
        let config: Self = toml::from_str(contents).map_err(|error| GromaqError::ConfigParse {
            message: error.to_string(),
        })?;
        config.validate()?;
        Ok(config)
    }

    /// Serialize this configuration as TOML after validating it.
    pub fn to_toml_string(&self) -> Result<String> {
        self.validate()?;
        toml::to_string_pretty(self).map_err(|error| GromaqError::ConfigSerialize {
            message: error.to_string(),
        })
    }

    /// Validate configuration values.
    pub fn validate(&self) -> Result<()> {
        validate_terminal_dimensions(self.terminal.cols, self.terminal.rows)?;
        if self.terminal.scrollback_lines > 1_000_000 {
            return Err(GromaqError::InvalidScrollback {
                maximum: 1_000_000,
                actual: self.terminal.scrollback_lines,
            });
        }
        if !self.font.size_px.is_finite()
            || !(MIN_FONT_SIZE_PX..=MAX_FONT_SIZE_PX).contains(&self.font.size_px)
        {
            return Err(GromaqError::InvalidFontSize {
                minimum: MIN_FONT_SIZE_PX,
                maximum: MAX_FONT_SIZE_PX,
                actual: self.font.size_px,
            });
        }
        if self.performance.target_fps == 0 {
            return Err(GromaqError::InvalidTargetFps {
                minimum: 1,
                actual: self.performance.target_fps,
            });
        }
        validate_shell_settings(&self.shell)?;
        Ok(())
    }
}

fn validate_shell_settings(shell: &ShellSettings) -> Result<()> {
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
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            family: "monospace".to_owned(),
            size_px: 14.0,
        }
    }
}

impl FontSettings {
    /// Deterministic renderer font size used for glyph cache keys and render planning.
    pub fn renderer_font_size_px(&self) -> u16 {
        self.size_px.round() as u16
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
