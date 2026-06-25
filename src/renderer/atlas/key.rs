use crate::cell::{Color, Style, UnderlineStyle};

/// Stable glyph cache text identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GlyphKeyText {
    /// A single scalar value.
    Scalar(char),
    /// A multi-scalar terminal cell text cluster.
    Cluster(String),
}

/// Stable glyph cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    /// Text to render.
    pub text: GlyphKeyText,
    /// Cell style.
    pub style: Style,
    /// Font size in pixels.
    pub font_size_px: u16,
}

impl GlyphKey {
    /// Build a glyph cache key.
    pub fn new(ch: char, style: Style, font_size_px: u16) -> Self {
        Self {
            text: GlyphKeyText::Scalar(ch),
            style: glyph_raster_style(style),
            font_size_px,
        }
    }

    /// Build a glyph cache key for a full terminal cell text cluster.
    pub fn for_text(text: &str, first_char: char, style: Style, font_size_px: u16) -> Self {
        if text.len() == first_char.len_utf8() {
            Self::new(first_char, style, font_size_px)
        } else {
            Self {
                text: GlyphKeyText::Cluster(text.to_owned()),
                style: glyph_raster_style(style),
                font_size_px,
            }
        }
    }
}

fn glyph_raster_style(style: Style) -> Style {
    Style {
        foreground: Color::Default,
        background: Color::Default,
        dim: false,
        underline: false,
        underline_style: UnderlineStyle::Single,
        underline_color_id: 0,
        blink: false,
        hidden: false,
        inverse: false,
        overline: false,
        strikethrough: false,
        framed: false,
        encircled: false,
        ..style
    }
}
