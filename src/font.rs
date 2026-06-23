//! Font-backed glyph rasterization.

use std::collections::HashMap;

use swash::scale::image::Content;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::Format;
use swash::{CacheKey, FontRef};
use thiserror::Error;

use crate::renderer::{GlyphBitmap, GlyphEntry, RenderPlan};

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
        })
    }

    /// Rasterize one character at the requested pixel size into an atlas bitmap.
    pub fn rasterize(
        &mut self,
        ch: char,
        size_px: f32,
        entry: GlyphEntry,
    ) -> Result<GlyphBitmap, FontRasterError> {
        let font = self.font_ref();
        let glyph_id = font.charmap().map(ch);
        if glyph_id == 0 {
            return Err(FontRasterError::MissingGlyph(ch));
        }

        let mut context = ScaleContext::new();
        let mut scaler = context.builder(font).size(size_px).hint(true).build();
        let image = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .render(&mut scaler, glyph_id)
        .ok_or(FontRasterError::RenderFailed(ch))?;

        let width = image.placement.width;
        let height = image.placement.height;
        let rgba = image_to_rgba8(image.content, width, height, &image.data)?;
        Ok(GlyphBitmap {
            entry,
            width,
            height,
            rgba,
        })
    }

    fn font_ref(&self) -> FontRef<'_> {
        FontRef {
            data: &self.font_bytes,
            offset: self.offset,
            key: self.key,
        }
    }
}

/// Rasterized glyph bitmaps needed to draw a render plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasterizedGlyphBatch {
    /// Distinct glyph bitmaps referenced by the render plan.
    pub bitmaps: Vec<GlyphBitmap>,
    /// Count of glyphs rasterized during this batch.
    pub rasterized: usize,
    /// Count of planned glyphs already available in the cache.
    pub reused: usize,
}

/// Cache that turns planned glyph draw commands into real font-backed atlas bitmaps.
pub struct RasterizedGlyphCache {
    rasterizer: FontRasterizer,
    bitmaps: HashMap<GlyphEntry, GlyphBitmap>,
}

impl RasterizedGlyphCache {
    /// Build a glyph bitmap cache from font or font-collection bytes.
    pub fn from_bytes(font_bytes: Vec<u8>) -> Result<Self, FontRasterError> {
        Ok(Self {
            rasterizer: FontRasterizer::from_bytes(font_bytes)?,
            bitmaps: HashMap::new(),
        })
    }

    /// Rasterize every distinct planned glyph missing from the cache.
    pub fn rasterize_plan(
        &mut self,
        plan: &RenderPlan,
    ) -> Result<RasterizedGlyphBatch, FontRasterError> {
        let mut bitmaps = Vec::new();
        let mut rasterized = 0;
        let mut reused = 0;

        for glyph in &plan.glyphs {
            if let Some(bitmap) = self.bitmaps.get(&glyph.atlas_entry) {
                reused += 1;
                if !bitmaps
                    .iter()
                    .any(|existing: &GlyphBitmap| existing.entry == glyph.atlas_entry)
                {
                    bitmaps.push(bitmap.clone());
                }
                continue;
            }
            self.bitmaps
                .retain(|entry, _| entry.slot != glyph.atlas_entry.slot);
            let bitmap = self.rasterizer.rasterize(
                glyph.ch,
                f32::from(glyph.font_size_px),
                glyph.atlas_entry,
            )?;
            self.bitmaps.insert(glyph.atlas_entry, bitmap.clone());
            if !bitmaps
                .iter()
                .any(|existing: &GlyphBitmap| existing.entry == glyph.atlas_entry)
            {
                bitmaps.push(bitmap);
            }
            rasterized += 1;
        }

        Ok(RasterizedGlyphBatch {
            bitmaps,
            rasterized,
            reused,
        })
    }

    /// Number of cached glyph bitmaps.
    pub fn len(&self) -> usize {
        self.bitmaps.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.bitmaps.is_empty()
    }
}

fn image_to_rgba8(
    content: Content,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<Vec<u8>, FontRasterError> {
    let pixel_count = usize::try_from(width)
        .unwrap_or(usize::MAX)
        .saturating_mul(usize::try_from(height).unwrap_or(usize::MAX));
    match content {
        Content::Mask => {
            if data.len() != pixel_count {
                return Err(FontRasterError::InvalidImageBuffer {
                    width,
                    height,
                    content,
                });
            }
            let mut rgba = Vec::with_capacity(pixel_count.saturating_mul(4));
            for alpha in data {
                rgba.extend_from_slice(&[255, 255, 255, *alpha]);
            }
            Ok(rgba)
        }
        Content::SubpixelMask | Content::Color => {
            let expected_len = pixel_count.saturating_mul(4);
            if data.len() != expected_len {
                return Err(FontRasterError::InvalidImageBuffer {
                    width,
                    height,
                    content,
                });
            }
            Ok(data.to_vec())
        }
    }
}
