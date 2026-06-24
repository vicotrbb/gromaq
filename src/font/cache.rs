//! Render-plan glyph bitmap cache backed by font rasterizers.

use std::collections::HashMap;

use super::{FontRasterError, FontRasterizer};
use crate::renderer::{GlyphBitmap, GlyphEntry, RenderPlan};

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
