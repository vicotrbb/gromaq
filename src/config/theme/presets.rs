//! Built-in theme preset definitions.

use serde::{Deserialize, Serialize};

use super::{
    CursorStyleSetting, DEFAULT_ANSI_COLORS, DEFAULT_BACKGROUND, DEFAULT_BACKGROUND_OPACITY,
    DEFAULT_CELL_SPACING_PX, DEFAULT_CURSOR, DEFAULT_DIM_OPACITY, DEFAULT_FOREGROUND,
    DEFAULT_SELECTION, DEFAULT_SURFACE_PADDING_PX, ThemeSettings,
};

/// Name of the original polished dark terminal theme.
pub const DARK_THEME_PRESET: &str = "gromaq-dark";
/// Name of the alternate high-contrast graphite theme.
pub const GRAPHITE_THEME_PRESET: &str = "gromaq-graphite";
/// Name of the Ghostty-inspired dark terminal theme.
pub const GHOSTTY_THEME_PRESET: &str = "gromaq-ghostty";

/// Named built-in terminal theme preset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePresetSetting {
    /// Polished dark theme tuned for legibility and native terminal screenshots.
    GromaqDark,
    /// Cooler graphite theme with a brighter foreground and crisp ANSI colors.
    GromaqGraphite,
    /// Ghostty-inspired dark theme with calm contrast and expressive ANSI colors.
    #[default]
    GromaqGhostty,
}

impl ThemeSettings {
    /// Return the complete built-in theme represented by a named preset.
    pub fn from_preset(preset: ThemePresetSetting) -> Self {
        match preset {
            ThemePresetSetting::GromaqDark => Self {
                preset,
                background: "#171b24".to_owned(),
                foreground: "#edf3fb".to_owned(),
                cursor: DEFAULT_CURSOR.to_owned(),
                selection: "#33445f".to_owned(),
                background_opacity: DEFAULT_BACKGROUND_OPACITY,
                cursor_style: CursorStyleSetting::default(),
                cursor_blinking: true,
                ansi: [
                    "#2a2f3a", "#ff6b7a", "#8bdc8b", "#f6c177", "#8aadf4", "#c6a0f6", "#8bd5ca",
                    "#cad3e3", "#6e7686", "#ff8fa3", "#a6e3a1", "#f9d58a", "#a6c8ff", "#f5bde6",
                    "#9ee7dc", "#f7fbff",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
                surface_padding_px: DEFAULT_SURFACE_PADDING_PX,
                cell_spacing_px: DEFAULT_CELL_SPACING_PX,
                dim_opacity: 0.66,
            },
            ThemePresetSetting::GromaqGraphite => Self {
                preset,
                background: "#0b0f14".to_owned(),
                foreground: "#f3f6fb".to_owned(),
                cursor: "#ffd166".to_owned(),
                selection: "#26445f".to_owned(),
                background_opacity: DEFAULT_BACKGROUND_OPACITY,
                cursor_style: CursorStyleSetting::default(),
                cursor_blinking: true,
                ansi: [
                    "#1f2630", "#ff6b7a", "#8fd694", "#ffd166", "#8ab4ff", "#cba6f7", "#7dd3c7",
                    "#d7deea", "#6b7280", "#ff8fa3", "#a7e3a1", "#ffe08a", "#b6ccff", "#f5bde6",
                    "#9be4d8", "#ffffff",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
                surface_padding_px: DEFAULT_SURFACE_PADDING_PX,
                cell_spacing_px: DEFAULT_CELL_SPACING_PX,
                dim_opacity: 0.7,
            },
            ThemePresetSetting::GromaqGhostty => Self {
                preset,
                background: DEFAULT_BACKGROUND.to_owned(),
                foreground: DEFAULT_FOREGROUND.to_owned(),
                cursor: DEFAULT_CURSOR.to_owned(),
                selection: DEFAULT_SELECTION.to_owned(),
                background_opacity: DEFAULT_BACKGROUND_OPACITY,
                cursor_style: CursorStyleSetting::default(),
                cursor_blinking: true,
                ansi: DEFAULT_ANSI_COLORS
                    .iter()
                    .map(|color| (*color).to_owned())
                    .collect(),
                surface_padding_px: DEFAULT_SURFACE_PADDING_PX,
                cell_spacing_px: DEFAULT_CELL_SPACING_PX,
                dim_opacity: DEFAULT_DIM_OPACITY,
            },
        }
    }
}

/// Serialize a theme preset as user-facing TOML text.
pub fn format_theme_preset(preset: ThemePresetSetting) -> &'static str {
    match preset {
        ThemePresetSetting::GromaqDark => DARK_THEME_PRESET,
        ThemePresetSetting::GromaqGraphite => GRAPHITE_THEME_PRESET,
        ThemePresetSetting::GromaqGhostty => GHOSTTY_THEME_PRESET,
    }
}

/// Parse a user-facing TOML theme preset name.
pub fn parse_theme_preset(value: &str) -> Option<ThemePresetSetting> {
    match value {
        DARK_THEME_PRESET => Some(ThemePresetSetting::GromaqDark),
        GRAPHITE_THEME_PRESET => Some(ThemePresetSetting::GromaqGraphite),
        GHOSTTY_THEME_PRESET => Some(ThemePresetSetting::GromaqGhostty),
        _ => None,
    }
}
