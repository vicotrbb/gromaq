//! User configuration model and validation.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{GromaqError, Result};

mod reload;
mod settings;
mod theme;

pub use reload::{ConfigFileReloader, ConfigReload};
pub use settings::{
    FontSettings, MAX_FONT_SIZE_PX, MAX_TARGET_FPS, MAX_TERMINAL_CELLS, MIN_FONT_SIZE_PX,
    PerformanceSettings, ShellSettings, TerminalSettings,
};
pub use theme::{
    ANSI_COLOR_COUNT, CursorStyleSetting, DEFAULT_ANSI_COLORS, DEFAULT_ANSI_COLORS_RGB8,
    DEFAULT_BACKGROUND, DEFAULT_BACKGROUND_RGB8, DEFAULT_CURSOR, DEFAULT_CURSOR_RGB8,
    DEFAULT_FOREGROUND, DEFAULT_FOREGROUND_RGB8, DEFAULT_SELECTION, DEFAULT_SELECTION_RGB8,
    DEFAULT_SURFACE_PADDING_PX, ThemeSettings,
};

pub(crate) use settings::validate_terminal_dimensions;

use settings::validate_shell_settings;

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
    /// Visual theme colors.
    pub theme: ThemeSettings,
    /// Performance-related targets.
    pub performance: PerformanceSettings,
}

impl GromaqConfig {
    /// Load, parse, and validate a TOML configuration file from disk.
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        let contents = read_config_contents(path.as_ref())?;
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
        if !(1..=MAX_TARGET_FPS).contains(&self.performance.target_fps) {
            return Err(GromaqError::InvalidTargetFps {
                minimum: 1,
                maximum: MAX_TARGET_FPS,
                actual: self.performance.target_fps,
            });
        }
        self.theme.validate()?;
        validate_shell_settings(&self.shell)?;
        Ok(())
    }
}

fn read_config_contents(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|error| GromaqError::ConfigRead {
        path: path.display().to_string(),
        message: error.to_string(),
    })
}
