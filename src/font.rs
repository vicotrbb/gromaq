//! Font-backed glyph rasterization.

use std::collections::HashMap;

use swash::scale::image::Content;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::shape::ShapeContext;
use swash::zeno::Format;
use swash::{CacheKey, FontRef, GlyphId};
use thiserror::Error;
use unicode_width::UnicodeWidthChar;

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderedGlyph {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn compose_rendered_glyphs(entry: GlyphEntry, glyphs: &[RenderedGlyph]) -> Option<GlyphBitmap> {
    let first = glyphs.first()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first
        .x
        .saturating_add(i32::try_from(first.width).unwrap_or(i32::MAX));
    let mut max_y = first
        .y
        .saturating_add(i32::try_from(first.height).unwrap_or(i32::MAX));

    for glyph in &glyphs[1..] {
        min_x = min_x.min(glyph.x);
        min_y = min_y.min(glyph.y);
        max_x = max_x.max(
            glyph
                .x
                .saturating_add(i32::try_from(glyph.width).unwrap_or(i32::MAX)),
        );
        max_y = max_y.max(
            glyph
                .y
                .saturating_add(i32::try_from(glyph.height).unwrap_or(i32::MAX)),
        );
    }

    let width = u32::try_from(max_x.saturating_sub(min_x)).ok()?;
    let height = u32::try_from(max_y.saturating_sub(min_y)).ok()?;
    if width == 0 || height == 0 {
        return None;
    }

    let mut rgba = vec![
        0;
        usize::try_from(width)
            .unwrap_or(usize::MAX)
            .saturating_mul(usize::try_from(height).unwrap_or(usize::MAX))
            .saturating_mul(4)
    ];
    for glyph in glyphs {
        blend_glyph_into_canvas(glyph, min_x, min_y, width, &mut rgba);
    }

    Some(GlyphBitmap {
        entry,
        width,
        height,
        rgba,
    })
}

fn blend_glyph_into_canvas(
    glyph: &RenderedGlyph,
    min_x: i32,
    min_y: i32,
    canvas_width: u32,
    canvas: &mut [u8],
) {
    let offset_x = u32::try_from(glyph.x.saturating_sub(min_x)).unwrap_or(0);
    let offset_y = u32::try_from(glyph.y.saturating_sub(min_y)).unwrap_or(0);
    for source_y in 0..glyph.height {
        for source_x in 0..glyph.width {
            let source_index =
                usize::try_from((source_y * glyph.width + source_x) * 4).unwrap_or(usize::MAX);
            let target_index =
                usize::try_from(((offset_y + source_y) * canvas_width + offset_x + source_x) * 4)
                    .unwrap_or(usize::MAX);
            if source_index + 3 >= glyph.rgba.len() || target_index + 3 >= canvas.len() {
                continue;
            }
            let source_alpha = glyph.rgba[source_index + 3];
            if source_alpha > canvas[target_index + 3] {
                canvas[target_index..target_index + 4]
                    .copy_from_slice(&glyph.rgba[source_index..source_index + 4]);
            }
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
    rasterizers: Vec<FontRasterizer>,
    bitmaps: HashMap<GlyphEntry, GlyphBitmap>,
}

impl RasterizedGlyphCache {
    /// Build a glyph bitmap cache from font or font-collection bytes.
    pub fn from_bytes(font_bytes: Vec<u8>) -> Result<Self, FontRasterError> {
        Self::from_font_bytes(vec![font_bytes])
    }

    /// Build a glyph bitmap cache from an ordered primary/fallback font stack.
    pub fn from_font_bytes(font_bytes: Vec<Vec<u8>>) -> Result<Self, FontRasterError> {
        if font_bytes.is_empty() {
            return Err(FontRasterError::InvalidFont);
        }
        Ok(Self {
            rasterizers: font_bytes
                .into_iter()
                .map(FontRasterizer::from_bytes)
                .collect::<Result<Vec<_>, _>>()?,
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
            let bitmap = self.rasterize_text_with_fallback(
                &glyph.text,
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

    fn rasterize_text_with_fallback(
        &mut self,
        text: &str,
        size_px: f32,
        entry: GlyphEntry,
    ) -> Result<GlyphBitmap, FontRasterError> {
        let mut missing = None;
        for rasterizer in &mut self.rasterizers {
            match rasterizer.rasterize_text(text, size_px, entry) {
                Ok(bitmap) => return Ok(bitmap),
                Err(FontRasterError::MissingGlyph(ch)) => missing = Some(ch),
                Err(error) => return Err(error),
            }
        }
        Err(FontRasterError::MissingGlyph(missing.unwrap_or('\0')))
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
