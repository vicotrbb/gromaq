//! Built-in theme preset definitions.

use serde::{Deserialize, Serialize};

use super::{
    CursorStyleSetting, DEFAULT_ANSI_COLORS, DEFAULT_BACKGROUND, DEFAULT_CURSOR,
    DEFAULT_DIM_OPACITY, DEFAULT_FOREGROUND, DEFAULT_SELECTION, DEFAULT_SURFACE_PADDING_PX,
    DEFAULT_THEME_PRESET, ThemeSettings,
};

/// Name of the alternate high-contrast graphite theme.
pub const GRAPHITE_THEME_PRESET: &str = "gromaq-graphite";

/// Named built-in terminal theme preset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePresetSetting {
    /// Polished dark theme tuned for legibility and native terminal screenshots.
    #[default]
    GromaqDark,
    /// Cooler graphite theme with a brighter foreground and crisp ANSI colors.
    GromaqGraphite,
}

impl ThemeSettings {
    /// Return the complete built-in theme represented by a named preset.
    pub fn from_preset(preset: ThemePresetSetting) -> Self {
        match preset {
            ThemePresetSetting::GromaqDark => Self {
                preset,
                background: DEFAULT_BACKGROUND.to_owned(),
                foreground: DEFAULT_FOREGROUND.to_owned(),
                cursor: DEFAULT_CURSOR.to_owned(),
                selection: DEFAULT_SELECTION.to_owned(),
                cursor_style: CursorStyleSetting::default(),
                cursor_blinking: true,
                ansi: DEFAULT_ANSI_COLORS
                    .iter()
                    .map(|color| (*color).to_owned())
                    .collect(),
                surface_padding_px: DEFAULT_SURFACE_PADDING_PX,
                dim_opacity: DEFAULT_DIM_OPACITY,
            },
            ThemePresetSetting::GromaqGraphite => Self {
                preset,
                background: "#0b0f14".to_owned(),
                foreground: "#f3f6fb".to_owned(),
                cursor: "#ffd166".to_owned(),
                selection: "#26445f".to_owned(),
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
                dim_opacity: 0.7,
            },
        }
    }
}

/// Serialize a theme preset as user-facing TOML text.
pub fn format_theme_preset(preset: ThemePresetSetting) -> &'static str {
    match preset {
        ThemePresetSetting::GromaqDark => DEFAULT_THEME_PRESET,
        ThemePresetSetting::GromaqGraphite => GRAPHITE_THEME_PRESET,
    }
}
