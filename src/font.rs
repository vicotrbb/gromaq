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
    let mut max_x = checked_glyph_edge(first.x, first.width)?;
    let mut max_y = checked_glyph_edge(first.y, first.height)?;

    for glyph in &glyphs[1..] {
        min_x = min_x.min(glyph.x);
        min_y = min_y.min(glyph.y);
        max_x = max_x.max(checked_glyph_edge(glyph.x, glyph.width)?);
        max_y = max_y.max(checked_glyph_edge(glyph.y, glyph.height)?);
    }

    let width = checked_glyph_span(min_x, max_x)?;
    let height = checked_glyph_span(min_y, max_y)?;
    if width == 0 || height == 0 {
        return None;
    }

    let mut rgba = zeroed_rgba_buffer(width, height)?;
    for glyph in glyphs {
        blend_glyph_into_canvas(glyph, min_x, min_y, width, &mut rgba)?;
    }

    Some(GlyphBitmap {
        entry,
        width,
        height,
        rgba,
    })
}

fn checked_glyph_edge(origin: i32, extent: u32) -> Option<i32> {
    origin.checked_add(i32::try_from(extent).ok()?)
}

fn checked_glyph_span(min: i32, max: i32) -> Option<u32> {
    u32::try_from(max.checked_sub(min)?).ok()
}

fn checked_glyph_canvas_offset(position: i32, origin: i32) -> Option<u32> {
    u32::try_from(position.checked_sub(origin)?).ok()
}

fn blend_glyph_into_canvas(
    glyph: &RenderedGlyph,
    min_x: i32,
    min_y: i32,
    canvas_width: u32,
    canvas: &mut [u8],
) -> Option<()> {
    let offset_x = checked_glyph_canvas_offset(glyph.x, min_x)?;
    let offset_y = checked_glyph_canvas_offset(glyph.y, min_y)?;
    for source_y in 0..glyph.height {
        for source_x in 0..glyph.width {
            let Some(source_index) = rgba_offset(glyph.width, source_x, source_y) else {
                continue;
            };
            let Some(target_x) = offset_x.checked_add(source_x) else {
                continue;
            };
            let Some(target_y) = offset_y.checked_add(source_y) else {
                continue;
            };
            let Some(target_index) = rgba_offset(canvas_width, target_x, target_y) else {
                continue;
            };
            let source_end = source_index.checked_add(4)?;
            let target_end = target_index.checked_add(4)?;
            if source_end > glyph.rgba.len() || target_end > canvas.len() {
                return None;
            }
            let source_alpha = glyph.rgba[source_index + 3];
            if source_alpha > canvas[target_index + 3] {
                canvas[target_index..target_end]
                    .copy_from_slice(&glyph.rgba[source_index..source_end]);
            }
        }
    }
    Some(())
}

fn rgba_pixel_count(width: u32, height: u32) -> Option<usize> {
    usize::try_from(width).ok().and_then(|width| {
        usize::try_from(height)
            .ok()
            .and_then(|height| width.checked_mul(height))
    })
}

fn rgba_byte_len(width: u32, height: u32) -> Option<usize> {
    rgba_pixel_count(width, height).and_then(|pixels| pixels.checked_mul(4))
}

fn zeroed_rgba_buffer(width: u32, height: u32) -> Option<Vec<u8>> {
    let len = rgba_byte_len(width, height)?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(len).ok()?;
    rgba.resize(len, 0);
    Some(rgba)
}

fn rgba_offset(width: u32, x: u32, y: u32) -> Option<usize> {
    usize::try_from(y)
        .ok()
        .and_then(|y| {
            usize::try_from(width)
                .ok()
                .and_then(|width| y.checked_mul(width))
        })
        .and_then(|row_start| {
            usize::try_from(x)
                .ok()
                .and_then(|x| row_start.checked_add(x))
        })
        .and_then(|pixel_offset| pixel_offset.checked_mul(4))
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
    let pixel_count =
        rgba_pixel_count(width, height).ok_or(FontRasterError::InvalidImageBuffer {
            width,
            height,
            content,
        })?;
    match content {
        Content::Mask => {
            if data.len() != pixel_count {
                return Err(FontRasterError::InvalidImageBuffer {
                    width,
                    height,
                    content,
                });
            }
            let expected_len =
                rgba_byte_len(width, height).ok_or(FontRasterError::InvalidImageBuffer {
                    width,
                    height,
                    content,
                })?;
            let mut rgba = Vec::new();
            rgba.try_reserve_exact(expected_len).map_err(|_| {
                FontRasterError::InvalidImageBuffer {
                    width,
                    height,
                    content,
                }
            })?;
            for alpha in data {
                rgba.extend_from_slice(&[255, 255, 255, *alpha]);
            }
            Ok(rgba)
        }
        Content::SubpixelMask | Content::Color => {
            let expected_len =
                rgba_byte_len(width, height).ok_or(FontRasterError::InvalidImageBuffer {
                    width,
                    height,
                    content,
                })?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_to_rgba8_rejects_oversized_mask_dimensions_before_allocation() {
        let error = image_to_rgba8(Content::Mask, u32::MAX, u32::MAX, &[]).unwrap_err();

        assert_eq!(
            error,
            FontRasterError::InvalidImageBuffer {
                width: u32::MAX,
                height: u32::MAX,
                content: Content::Mask,
            }
        );
    }

    #[test]
    fn image_to_rgba8_rejects_oversized_color_dimensions_before_allocation() {
        let error = image_to_rgba8(Content::Color, u32::MAX, u32::MAX, &[]).unwrap_err();

        assert_eq!(
            error,
            FontRasterError::InvalidImageBuffer {
                width: u32::MAX,
                height: u32::MAX,
                content: Content::Color,
            }
        );
    }

    #[test]
    fn glyph_geometry_helpers_reject_overflowing_bounds() {
        assert_eq!(checked_glyph_edge(-2, 4), Some(2));
        assert_eq!(checked_glyph_edge(i32::MAX, 1), None);
        assert_eq!(checked_glyph_span(-2, 2), Some(4));
        assert_eq!(checked_glyph_span(2, -2), None);
        assert_eq!(checked_glyph_canvas_offset(5, 2), Some(3));
        assert_eq!(checked_glyph_canvas_offset(2, 5), None);
    }

    #[test]
    fn compose_rendered_glyphs_rejects_overflowing_bounds() {
        let glyph = RenderedGlyph {
            x: i32::MAX,
            y: 0,
            width: 1,
            height: 1,
            rgba: vec![255, 255, 255, 255],
        };

        assert_eq!(
            compose_rendered_glyphs(
                GlyphEntry {
                    slot: 0,
                    generation: 0,
                },
                &[glyph],
            ),
            None
        );
    }

    #[test]
    fn compose_rendered_glyphs_rejects_truncated_source_rgba() {
        let glyph = RenderedGlyph {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            rgba: vec![255, 255, 255],
        };

        assert_eq!(
            compose_rendered_glyphs(
                GlyphEntry {
                    slot: 0,
                    generation: 0,
                },
                &[glyph],
            ),
            None
        );
    }
}
