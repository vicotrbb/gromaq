//! Font-backed glyph rasterization.

use swash::scale::image::Content;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::shape::ShapeContext;
use swash::zeno::Format;
use swash::{CacheKey, FontRef, GlyphId};
use thiserror::Error;
use unicode_width::UnicodeWidthChar;

use crate::renderer::{GlyphBitmap, GlyphEntry};

mod cache;
mod image;

pub use cache::{RasterizedGlyphBatch, RasterizedGlyphCache};
use image::{RenderedGlyph, compose_rendered_glyphs, image_to_rgba8};

/// Errors produced by font-backed glyph rasterization.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FontRasterError {
    /// The supplied bytes are not a readable font or font collection.
    #[error("invalid font bytes")]
    InvalidFont,
    /// The requested character does not map to a glyph in the font.
    #[error("font does not contain glyph for {0:?}")]
    MissingGlyph(char),
    /// The glyph could not be rendered to an image.
    #[error("could not render glyph for {0:?}")]
    RenderFailed(char),
    /// The rendered image buffer size does not match the reported placement.
    #[error("glyph image buffer length did not match {width}x{height} {content:?} image")]
    InvalidImageBuffer {
        /// Rendered image width in pixels.
        width: u32,
        /// Rendered image height in pixels.
        height: u32,
        /// Rendered image content type.
        content: Content,
    },
}

/// Swash-backed rasterizer that owns font bytes and renders glyphs into RGBA8 bitmaps.
pub struct FontRasterizer {
    font_bytes: Vec<u8>,
    offset: u32,
    key: CacheKey,
    shape_context: ShapeContext,
    scale_context: ScaleContext,
}

impl FontRasterizer {
    /// Build a rasterizer from font or font-collection bytes.
    pub fn from_bytes(font_bytes: Vec<u8>) -> Result<Self, FontRasterError> {
        let (offset, key) = {
            let font = FontRef::from_index(&font_bytes, 0).ok_or(FontRasterError::InvalidFont)?;
            (font.offset, font.key)
        };
        Ok(Self {
            font_bytes,
            offset,
            key,
            shape_context: ShapeContext::new(),
            scale_context: ScaleContext::new(),
        })
    }

    /// Rasterize one character at the requested pixel size into an atlas bitmap.
    pub fn rasterize(
        &mut self,
        ch: char,
        size_px: f32,
        entry: GlyphEntry,
    ) -> Result<GlyphBitmap, FontRasterError> {
        self.rasterize_text(&ch.to_string(), size_px, entry)
    }

    /// Rasterize one terminal cell text cluster at the requested pixel size.
    pub fn rasterize_text(
        &mut self,
        text: &str,
        size_px: f32,
        entry: GlyphEntry,
    ) -> Result<GlyphBitmap, FontRasterError> {
        let first_char = text
            .chars()
            .next()
            .ok_or(FontRasterError::RenderFailed('\0'))?;
        if let Some(ch) = self.missing_visible_char(text) {
            return Err(FontRasterError::MissingGlyph(ch));
        }
        let glyphs = self.shape_text(text, size_px);
        if glyphs.is_empty() {
            return Err(FontRasterError::MissingGlyph(first_char));
        }

        let font = FontRef {
            data: &self.font_bytes,
            offset: self.offset,
            key: self.key,
        };
        let mut scaler = self
            .scale_context
            .builder(font)
            .size(size_px)
            .hint(true)
            .build();
        let mut renderer = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ]);
        let renderer = renderer.format(Format::Alpha);
        let mut rendered = Vec::with_capacity(glyphs.len());
        for glyph in glyphs {
            let image = renderer
                .render(&mut scaler, glyph.id)
                .ok_or(FontRasterError::RenderFailed(first_char))?;
            if image.placement.width == 0 || image.placement.height == 0 {
                continue;
            }
            let x = (glyph.x + image.placement.left as f32).floor() as i32;
            let y = -(glyph.y + image.placement.top as f32).floor() as i32;
            let rgba = image_to_rgba8(
                image.content,
                image.placement.width,
                image.placement.height,
                &image.data,
            )?;
            rendered.push(RenderedGlyph {
                x,
                y,
                width: image.placement.width,
                height: image.placement.height,
                rgba,
            });
        }

        compose_rendered_glyphs(entry, &rendered).ok_or(FontRasterError::RenderFailed(first_char))
    }

    fn shape_text(&mut self, text: &str, size_px: f32) -> Vec<ShapedGlyph> {
        let font = FontRef {
            data: &self.font_bytes,
            offset: self.offset,
            key: self.key,
        };
        let mut shaper = self.shape_context.builder(font).size(size_px).build();
        shaper.add_str(text);

        let mut glyphs = Vec::new();
        let mut pen_x = 0.0;
        shaper.shape_with(|cluster| {
            for glyph in cluster.glyphs {
                glyphs.push(ShapedGlyph {
                    id: glyph.id,
                    x: pen_x + glyph.x,
                    y: glyph.y,
                });
                pen_x += glyph.advance;
            }
        });
        glyphs
    }

    fn missing_visible_char(&self, text: &str) -> Option<char> {
        let font = FontRef {
            data: &self.font_bytes,
            offset: self.offset,
            key: self.key,
        };
        let charmap = font.charmap();
        text.chars()
            .find(|ch| UnicodeWidthChar::width(*ch).unwrap_or(0) > 0 && charmap.map(*ch) == 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ShapedGlyph {
    id: GlyphId,
    x: f32,
    y: f32,
}
