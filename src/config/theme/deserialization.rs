use serde::{Deserialize, Deserializer};

use super::{CursorStyleSetting, ThemePresetSetting, ThemeSettings};

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
        if let Some(cell_spacing_px) = raw.cell_spacing_px {
            settings.cell_spacing_px = cell_spacing_px;
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
    cell_spacing_px: Option<u16>,
    dim_opacity: Option<f32>,
}
