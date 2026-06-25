//! Theme configuration and color parsing.

use serde::{Deserialize, Serialize};

mod color;
mod deserialization;
mod presets;
mod validation;

pub use presets::{
    GHOSTTY_THEME_PRESET, GRAPHITE_THEME_PRESET, ThemePresetSetting, format_theme_preset,
    parse_theme_preset,
};

/// Maximum supported visual surface padding in physical pixels.
pub const MAX_SURFACE_PADDING_PX: u16 = 512;
/// Maximum supported visual gap between adjacent cells in physical pixels.
pub const MAX_CELL_SPACING_PX: u16 = 32;
/// Minimum useful opacity for dim text.
pub const MIN_DIM_OPACITY: f32 = 0.1;
/// Maximum useful opacity for dim text.
pub const MAX_DIM_OPACITY: f32 = 1.0;
/// Minimum supported terminal background opacity.
pub const MIN_BACKGROUND_OPACITY: f32 = 0.0;
/// Maximum supported terminal background opacity.
pub const MAX_BACKGROUND_OPACITY: f32 = 1.0;
/// Built-in Ghostty-inspired theme background.
pub const DEFAULT_BACKGROUND: &str = "#101216";
/// Built-in Ghostty-inspired theme background as RGB8.
pub const DEFAULT_BACKGROUND_RGB8: [u8; 3] = [16, 18, 22];
/// Built-in Ghostty-inspired theme foreground.
pub const DEFAULT_FOREGROUND: &str = "#eef4fb";
/// Built-in Ghostty-inspired theme foreground as RGB8.
pub const DEFAULT_FOREGROUND_RGB8: [u8; 3] = [238, 244, 251];
/// Built-in Ghostty-inspired theme cursor.
pub const DEFAULT_CURSOR: &str = "#f6c177";
/// Built-in Ghostty-inspired theme cursor as RGB8.
pub const DEFAULT_CURSOR_RGB8: [u8; 3] = [246, 193, 119];
/// Built-in Ghostty-inspired theme selection background.
pub const DEFAULT_SELECTION: &str = "#2f3b52";
/// Built-in Ghostty-inspired theme selection background as RGB8.
pub const DEFAULT_SELECTION_RGB8: [u8; 3] = [47, 59, 82];
/// Number of configurable ANSI palette entries.
pub const ANSI_COLOR_COUNT: usize = 16;
/// Built-in Ghostty-inspired ANSI palette.
pub const DEFAULT_ANSI_COLORS: [&str; ANSI_COLOR_COUNT] = [
    "#242933", "#ff6b7a", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#c8d3e5",
    "#5f667a", "#ff8fa3", "#b9f27c", "#ffd98a", "#9dbdff", "#d7afff", "#9ee7ff", "#f7fbff",
];
/// Built-in Ghostty-inspired ANSI palette as RGB8.
pub const DEFAULT_ANSI_COLORS_RGB8: [[u8; 3]; ANSI_COLOR_COUNT] = [
    [36, 41, 51],
    [255, 107, 122],
    [158, 206, 106],
    [224, 175, 104],
    [122, 162, 247],
    [187, 154, 247],
    [125, 207, 255],
    [200, 211, 229],
    [95, 102, 122],
    [255, 143, 163],
    [185, 242, 124],
    [255, 217, 138],
    [157, 189, 255],
    [215, 175, 255],
    [158, 231, 255],
    [247, 251, 255],
];
/// Built-in visual breathing room around terminal cells.
pub const DEFAULT_SURFACE_PADDING_PX: u16 = 14;
/// Built-in gap between adjacent terminal cells.
pub const DEFAULT_CELL_SPACING_PX: u16 = 0;
/// Built-in opacity for SGR dim text.
pub const DEFAULT_DIM_OPACITY: f32 = 0.68;
/// Built-in terminal background opacity.
pub const DEFAULT_BACKGROUND_OPACITY: f32 = 1.0;
/// Built-in terminal cursor opacity.
pub const DEFAULT_CURSOR_OPACITY: f32 = 1.0;
/// Built-in selected-cell background opacity.
pub const DEFAULT_SELECTION_OPACITY: f32 = 1.0;
/// Name of the built-in default dark theme.
pub const DEFAULT_THEME_PRESET: &str = "gromaq-ghostty";
/// Configurable terminal cursor shape.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CursorStyleSetting {
    /// Filled cell cursor.
    #[default]
    Block,
    /// Underline cursor.
    Underline,
    /// Vertical bar cursor.
    Bar,
}

/// Theme section of the configuration file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThemeSettings {
    /// Named built-in theme preset used as the baseline for explicit overrides.
    pub preset: ThemePresetSetting,
    /// Terminal background color as `#RRGGBB`.
    pub background: String,
    /// Default foreground color as `#RRGGBB`.
    pub foreground: String,
    /// Cursor color as `#RRGGBB`.
    pub cursor: String,
    /// Selection background color as `#RRGGBB`.
    pub selection: String,
    /// Cursor opacity multiplier.
    pub cursor_opacity: f32,
    /// Selected-cell background opacity multiplier.
    pub selection_opacity: f32,
    /// Terminal background opacity multiplier.
    pub background_opacity: f32,
    /// Default cursor shape before shell escape sequences override it.
    pub cursor_style: CursorStyleSetting,
    /// Whether the default cursor requests blinking.
    pub cursor_blinking: bool,
    /// ANSI and bright ANSI colors as sixteen `#RRGGBB` entries.
    pub ansi: Vec<String>,
    /// Empty space around rendered terminal cells in physical pixels.
    pub surface_padding_px: u16,
    /// Optional visual gap between adjacent terminal cells in physical pixels.
    pub cell_spacing_px: u16,
    /// Opacity multiplier for SGR dim text.
    pub dim_opacity: f32,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self::from_preset(ThemePresetSetting::default())
    }
}
