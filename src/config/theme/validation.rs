use super::color::parse_hex_rgb;
use super::{
    ANSI_COLOR_COUNT, DEFAULT_ANSI_COLORS_RGB8, MAX_BACKGROUND_OPACITY, MAX_CELL_SPACING_PX,
    MAX_DIM_OPACITY, MAX_SURFACE_PADDING_PX, MIN_BACKGROUND_OPACITY, MIN_DIM_OPACITY,
    ThemeSettings,
};
use crate::error::{GromaqError, Result};

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
        if self.cell_spacing_px > MAX_CELL_SPACING_PX {
            return Err(GromaqError::InvalidThemeCellSpacing {
                maximum: MAX_CELL_SPACING_PX,
                actual: self.cell_spacing_px,
            });
        }
        if !self.background_opacity.is_finite()
            || !(MIN_BACKGROUND_OPACITY..=MAX_BACKGROUND_OPACITY).contains(&self.background_opacity)
        {
            return Err(GromaqError::InvalidThemeBackgroundOpacity {
                minimum: MIN_BACKGROUND_OPACITY,
                maximum: MAX_BACKGROUND_OPACITY,
                actual: self.background_opacity,
            });
        }
        validate_visible_opacity("cursor opacity", self.cursor_opacity)?;
        validate_visible_opacity("selection opacity", self.selection_opacity)?;
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

fn validate_visible_opacity(field: &'static str, opacity: f32) -> Result<()> {
    if !opacity.is_finite() || !(MIN_DIM_OPACITY..=MAX_DIM_OPACITY).contains(&opacity) {
        return Err(GromaqError::InvalidThemeOpacity {
            field,
            minimum: MIN_DIM_OPACITY,
            maximum: MAX_DIM_OPACITY,
            actual: opacity,
        });
    }
    Ok(())
}
