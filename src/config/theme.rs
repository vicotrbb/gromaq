//! Theme configuration and color parsing.

use serde::{Deserialize, Serialize};

use crate::error::{GromaqError, Result};

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
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            background: "#202127".to_owned(),
            foreground: "#e8e2d6".to_owned(),
            cursor: "#f4c06a".to_owned(),
        }
    }
}

impl ThemeSettings {
    /// Validate configured theme colors.
    pub fn validate(&self) -> Result<()> {
        self.background_rgb8()?;
        self.foreground_rgb8()?;
        self.cursor_rgb8()?;
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
