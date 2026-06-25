//! Theme configuration and color parsing.

use serde::{Deserialize, Deserializer, Serialize};

use crate::error::{GromaqError, Result};

mod presets;

pub use presets::{
    GHOSTTY_THEME_PRESET, GRAPHITE_THEME_PRESET, ThemePresetSetting, format_theme_preset,
};

/// Maximum supported visual surface padding in physical pixels.
pub const MAX_SURFACE_PADDING_PX: u16 = 512;
/// Minimum useful opacity for dim text.
pub const MIN_DIM_OPACITY: f32 = 0.1;
/// Maximum useful opacity for dim text.
pub const MAX_DIM_OPACITY: f32 = 1.0;
/// Built-in polished dark theme background.
pub const DEFAULT_BACKGROUND: &str = "#171b24";
/// Built-in polished dark theme background as RGB8.
pub const DEFAULT_BACKGROUND_RGB8: [u8; 3] = [23, 27, 36];
/// Built-in polished dark theme foreground.
pub const DEFAULT_FOREGROUND: &str = "#edf3fb";
/// Built-in polished dark theme foreground as RGB8.
pub const DEFAULT_FOREGROUND_RGB8: [u8; 3] = [237, 243, 251];
/// Built-in polished dark theme cursor.
pub const DEFAULT_CURSOR: &str = "#f6c177";
/// Built-in polished dark theme cursor as RGB8.
pub const DEFAULT_CURSOR_RGB8: [u8; 3] = [246, 193, 119];
/// Built-in polished dark theme selection background.
pub const DEFAULT_SELECTION: &str = "#33445f";
/// Built-in polished dark theme selection background as RGB8.
pub const DEFAULT_SELECTION_RGB8: [u8; 3] = [51, 68, 95];
/// Number of configurable ANSI palette entries.
pub const ANSI_COLOR_COUNT: usize = 16;
/// Built-in polished dark ANSI palette.
pub const DEFAULT_ANSI_COLORS: [&str; ANSI_COLOR_COUNT] = [
    "#2a2f3a", "#ff6b7a", "#8bdc8b", "#f6c177", "#8aadf4", "#c6a0f6", "#8bd5ca", "#cad3e3",
    "#6e7686", "#ff8fa3", "#a6e3a1", "#f9d58a", "#a6c8ff", "#f5bde6", "#9ee7dc", "#f7fbff",
];
/// Built-in polished dark ANSI palette as RGB8.
pub const DEFAULT_ANSI_COLORS_RGB8: [[u8; 3]; ANSI_COLOR_COUNT] = [
    [42, 47, 58],
    [255, 107, 122],
    [139, 220, 139],
    [246, 193, 119],
    [138, 173, 244],
    [198, 160, 246],
    [139, 213, 202],
    [202, 211, 227],
    [110, 118, 134],
    [255, 143, 163],
    [166, 227, 161],
    [249, 213, 138],
    [166, 200, 255],
    [245, 189, 230],
    [158, 231, 220],
    [247, 251, 255],
];
/// Built-in visual breathing room around terminal cells.
pub const DEFAULT_SURFACE_PADDING_PX: u16 = 14;
/// Built-in opacity for SGR dim text.
pub const DEFAULT_DIM_OPACITY: f32 = 0.66;
/// Name of the built-in default dark theme.
pub const DEFAULT_THEME_PRESET: &str = "gromaq-dark";
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
        Self::from_preset(ThemePresetSetting::default())
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

impl<'de> Deserialize<'de> for ThemeSettings {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawThemeSettings::deserialize(deserializer)?;
        let mut settings = ThemeSettings::from_preset(raw.preset);
        if let Some(background) = raw.background {
            settings.background = background;
        }
        if let Some(foreground) = raw.foreground {
            settings.foreground = foreground;
        }
        if let Some(cursor) = raw.cursor {
            settings.cursor = cursor;
        }
        if let Some(selection) = raw.selection {
            settings.selection = selection;
        }
        if let Some(cursor_style) = raw.cursor_style {
            settings.cursor_style = cursor_style;
        }
        if let Some(cursor_blinking) = raw.cursor_blinking {
            settings.cursor_blinking = cursor_blinking;
        }
        if let Some(ansi) = raw.ansi {
            settings.ansi = ansi;
        }
        if let Some(surface_padding_px) = raw.surface_padding_px {
            settings.surface_padding_px = surface_padding_px;
        }
        if let Some(dim_opacity) = raw.dim_opacity {
            settings.dim_opacity = dim_opacity;
        }
        Ok(settings)
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawThemeSettings {
    preset: ThemePresetSetting,
    background: Option<String>,
    foreground: Option<String>,
    cursor: Option<String>,
    selection: Option<String>,
    cursor_style: Option<CursorStyleSetting>,
    cursor_blinking: Option<bool>,
    ansi: Option<Vec<String>>,
    surface_padding_px: Option<u16>,
    dim_opacity: Option<f32>,
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
