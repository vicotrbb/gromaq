//! Theme configuration and color parsing.

use serde::{Deserialize, Serialize};

use crate::error::{GromaqError, Result};

/// Maximum supported visual surface padding in physical pixels.
pub const MAX_SURFACE_PADDING_PX: u16 = 512;
/// Minimum useful opacity for dim text.
pub const MIN_DIM_OPACITY: f32 = 0.1;
/// Maximum useful opacity for dim text.
pub const MAX_DIM_OPACITY: f32 = 1.0;
/// Built-in polished dark theme background.
pub const DEFAULT_BACKGROUND: &str = "#090d12";
/// Built-in polished dark theme background as RGB8.
pub const DEFAULT_BACKGROUND_RGB8: [u8; 3] = [9, 13, 18];
/// Built-in polished dark theme foreground.
pub const DEFAULT_FOREGROUND: &str = "#f4f7fb";
/// Built-in polished dark theme foreground as RGB8.
pub const DEFAULT_FOREGROUND_RGB8: [u8; 3] = [244, 247, 251];
/// Built-in polished dark theme cursor.
pub const DEFAULT_CURSOR: &str = "#ffd27a";
/// Built-in polished dark theme cursor as RGB8.
pub const DEFAULT_CURSOR_RGB8: [u8; 3] = [255, 210, 122];
/// Built-in polished dark theme selection background.
pub const DEFAULT_SELECTION: &str = "#2b4162";
/// Built-in polished dark theme selection background as RGB8.
pub const DEFAULT_SELECTION_RGB8: [u8; 3] = [43, 65, 98];
/// Number of configurable ANSI palette entries.
pub const ANSI_COLOR_COUNT: usize = 16;
/// Built-in polished dark ANSI palette.
pub const DEFAULT_ANSI_COLORS: [&str; ANSI_COLOR_COUNT] = [
    "#151922", "#ff6b7a", "#7ee787", "#ffd27a", "#82aaff", "#c792ea", "#7dcfff", "#d7dde8",
    "#6b7280", "#ff8fa3", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#ffffff",
];
/// Built-in polished dark ANSI palette as RGB8.
pub const DEFAULT_ANSI_COLORS_RGB8: [[u8; 3]; ANSI_COLOR_COUNT] = [
    [21, 25, 34],
    [255, 107, 122],
    [126, 231, 135],
    [255, 210, 122],
    [130, 170, 255],
    [199, 146, 234],
    [125, 207, 255],
    [215, 221, 232],
    [107, 114, 128],
    [255, 143, 163],
    [166, 227, 161],
    [249, 226, 175],
    [137, 180, 250],
    [245, 194, 231],
    [148, 226, 213],
    [255, 255, 255],
];
/// Built-in visual breathing room around terminal cells.
pub const DEFAULT_SURFACE_PADDING_PX: u16 = 14;
/// Built-in opacity for SGR dim text.
pub const DEFAULT_DIM_OPACITY: f32 = 0.66;
/// Name of the built-in default dark theme.
pub const DEFAULT_THEME_PRESET: &str = "gromaq-dark";

/// Named built-in terminal theme preset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePresetSetting {
    /// Polished dark theme tuned for legibility and native terminal screenshots.
    #[default]
    GromaqDark,
}

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
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
    /// Default cursor shape before shell escape sequences override it.
    pub cursor_style: CursorStyleSetting,
    /// Whether the default cursor requests blinking.
    pub cursor_blinking: bool,
    /// ANSI and bright ANSI colors as sixteen `#RRGGBB` entries.
    pub ansi: Vec<String>,
    /// Empty space around rendered terminal cells in physical pixels.
    pub surface_padding_px: u16,
    /// Opacity multiplier for SGR dim text.
    pub dim_opacity: f32,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            preset: ThemePresetSetting::default(),
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
        }
    }
}

impl ThemeSettings {
    /// Validate configured theme colors.
    pub fn validate(&self) -> Result<()> {
        self.background_rgb8()?;
        self.foreground_rgb8()?;
        self.cursor_rgb8()?;
        self.selection_rgb8()?;
        self.ansi_rgb8()?;
        if self.surface_padding_px > MAX_SURFACE_PADDING_PX {
            return Err(GromaqError::InvalidThemePadding {
                maximum: MAX_SURFACE_PADDING_PX,
                actual: self.surface_padding_px,
            });
        }
        if !self.dim_opacity.is_finite()
            || !(MIN_DIM_OPACITY..=MAX_DIM_OPACITY).contains(&self.dim_opacity)
        {
            return Err(GromaqError::InvalidThemeDimOpacity {
                minimum: MIN_DIM_OPACITY,
                maximum: MAX_DIM_OPACITY,
                actual: self.dim_opacity,
            });
        }
        Ok(())
    }

    /// Parsed background color.
    pub fn background_rgb8(&self) -> Result<[u8; 3]> {
        parse_hex_rgb("background", &self.background)
    }

    /// Parsed default foreground color.
    pub fn foreground_rgb8(&self) -> Result<[u8; 3]> {
        parse_hex_rgb("foreground", &self.foreground)
    }

    /// Parsed cursor color.
    pub fn cursor_rgb8(&self) -> Result<[u8; 3]> {
        parse_hex_rgb("cursor", &self.cursor)
    }

    /// Parsed selection background color.
    pub fn selection_rgb8(&self) -> Result<[u8; 3]> {
        parse_hex_rgb("selection", &self.selection)
    }

    /// Parsed ANSI color palette.
    pub fn ansi_rgb8(&self) -> Result<[[u8; 3]; ANSI_COLOR_COUNT]> {
        if self.ansi.len() != ANSI_COLOR_COUNT {
            return Err(GromaqError::InvalidThemeAnsiPaletteLength {
                expected: ANSI_COLOR_COUNT,
                actual: self.ansi.len(),
            });
        }
        let mut colors = DEFAULT_ANSI_COLORS_RGB8;
        for (index, value) in self.ansi.iter().enumerate() {
            colors[index] = parse_hex_rgb("ansi", value)?;
        }
        Ok(colors)
    }
}

/// Serialize a theme preset as user-facing TOML text.
pub fn format_theme_preset(preset: ThemePresetSetting) -> &'static str {
    match preset {
        ThemePresetSetting::GromaqDark => DEFAULT_THEME_PRESET,
    }
}

fn parse_hex_rgb(field: &'static str, value: &str) -> Result<[u8; 3]> {
    let Some(hex) = value.strip_prefix('#') else {
        return Err(invalid_theme_color(field, value));
    };
    if hex.len() != 6 || !hex.as_bytes().iter().all(u8::is_ascii_hexdigit) {
        return Err(invalid_theme_color(field, value));
    }
    let red = parse_hex_byte(field, value, &hex[0..2])?;
    let green = parse_hex_byte(field, value, &hex[2..4])?;
    let blue = parse_hex_byte(field, value, &hex[4..6])?;
    Ok([red, green, blue])
}

fn parse_hex_byte(field: &'static str, value: &str, byte: &str) -> Result<u8> {
    u8::from_str_radix(byte, 16).map_err(|_| invalid_theme_color(field, value))
}

fn invalid_theme_color(field: &'static str, value: &str) -> GromaqError {
    GromaqError::InvalidThemeColor {
        field,
        actual: value.to_owned(),
    }
}
