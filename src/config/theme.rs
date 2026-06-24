//! Theme configuration and color parsing.

use serde::{Deserialize, Serialize};

use crate::error::{GromaqError, Result};

/// Maximum supported visual surface padding in physical pixels.
pub const MAX_SURFACE_PADDING_PX: u16 = 512;
/// Built-in polished dark theme background.
pub const DEFAULT_BACKGROUND: &str = "#0b0f14";
/// Built-in polished dark theme background as RGB8.
pub const DEFAULT_BACKGROUND_RGB8: [u8; 3] = [11, 15, 20];
/// Built-in polished dark theme foreground.
pub const DEFAULT_FOREGROUND: &str = "#f2f4f8";
/// Built-in polished dark theme foreground as RGB8.
pub const DEFAULT_FOREGROUND_RGB8: [u8; 3] = [242, 244, 248];
/// Built-in polished dark theme cursor.
pub const DEFAULT_CURSOR: &str = "#f6c177";
/// Built-in polished dark theme cursor as RGB8.
pub const DEFAULT_CURSOR_RGB8: [u8; 3] = [246, 193, 119];
/// Built-in visual breathing room around terminal cells.
pub const DEFAULT_SURFACE_PADDING_PX: u16 = 14;

/// Theme section of the configuration file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeSettings {
    /// Terminal background color as `#RRGGBB`.
    pub background: String,
    /// Default foreground color as `#RRGGBB`.
    pub foreground: String,
    /// Cursor color as `#RRGGBB`.
    pub cursor: String,
    /// Empty space around rendered terminal cells in physical pixels.
    pub surface_padding_px: u16,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            background: DEFAULT_BACKGROUND.to_owned(),
            foreground: DEFAULT_FOREGROUND.to_owned(),
            cursor: DEFAULT_CURSOR.to_owned(),
            surface_padding_px: DEFAULT_SURFACE_PADDING_PX,
        }
    }
}

impl ThemeSettings {
    /// Validate configured theme colors.
    pub fn validate(&self) -> Result<()> {
        self.background_rgb8()?;
        self.foreground_rgb8()?;
        self.cursor_rgb8()?;
        if self.surface_padding_px > MAX_SURFACE_PADDING_PX {
            return Err(GromaqError::InvalidThemePadding {
                maximum: MAX_SURFACE_PADDING_PX,
                actual: self.surface_padding_px,
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
