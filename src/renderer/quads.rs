use thiserror::Error;

use super::atlas::GlyphEntry;
use super::color::style_foreground_rgba;
use super::{PlannedGlyph, RenderPlan};

mod background;
mod cursor;
mod text_decoration;

pub use background::{
    BackgroundQuad, BackgroundQuadBatch, BackgroundQuadConfig, BackgroundQuadError,
    BackgroundQuadPlanner, BackgroundVertex,
};
pub(in crate::renderer::quads) use background::{
    checked_background_quad_base_index, checked_background_quad_index_capacity,
};
pub use cursor::{CursorQuadConfig, CursorQuadPlanner};
pub use text_decoration::{TextDecorationQuadConfig, TextDecorationQuadPlanner};

/// Pixel and atlas layout used to build textured glyph quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphQuadConfig {
    /// Terminal cell width in pixels.
    pub cell_width_px: u32,
    /// Terminal cell height in pixels.
    pub cell_height_px: u32,
    /// Glyph atlas slot width in pixels.
    pub atlas_slot_width_px: u32,
    /// Glyph atlas slot height in pixels.
    pub atlas_slot_height_px: u32,
    /// Number of atlas slots per row.
    pub atlas_columns: u32,
    /// Atlas texture width in pixels.
    pub atlas_width_px: u32,
    /// Atlas texture height in pixels.
    pub atlas_height_px: u32,
}

/// Errors produced while building textured glyph quads.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GlyphQuadError {
    /// Pixel or atlas dimensions must be non-zero.
    #[error("glyph quad dimensions must be non-zero")]
    ZeroDimension,
    /// The planned glyph batch cannot be represented in `u32` GPU indices.
    #[error("glyph quad count is too large for u32 GPU indices")]
    IndexCountTooLarge,
    /// A glyph atlas slot falls outside the configured atlas texture.
    #[error("glyph atlas slot {slot} is outside the configured atlas image")]
    SlotOutsideAtlas {
        /// Atlas slot index that could not be represented.
        slot: u32,
    },
}

/// One vertex for a textured glyph quad.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphVertex {
    /// Pixel-space output position.
    pub position: [f32; 2],
    /// Atlas texture coordinate.
    pub uv: [f32; 2],
    /// Foreground text color in normalized RGBA.
    pub foreground_rgba: [f32; 4],
}

/// One textured glyph quad derived from a planned glyph.
#[derive(Debug, Clone, PartialEq)]
pub struct GlyphQuad {
    /// Full terminal cell text represented by this quad.
    pub text: String,
    /// Character represented by this quad.
    pub ch: char,
    /// Atlas entry sampled by this quad.
    pub atlas_entry: GlyphEntry,
    /// Quad vertices in top-left, top-right, bottom-right, bottom-left order.
    pub vertices: [GlyphVertex; 4],
}

/// Indexed glyph quad batch ready for GPU vertex/index buffer upload.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GlyphQuadBatch {
    /// Textured glyph quads.
    pub quads: Vec<GlyphQuad>,
    /// Triangle indices for all quads.
    pub indices: Vec<u32>,
}

/// Deterministic CPU-side planner for terminal glyph draw quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphQuadPlanner {
    config: GlyphQuadConfig,
}

impl GlyphQuadPlanner {
    /// Create a glyph quad planner.
    pub fn new(config: GlyphQuadConfig) -> Self {
        Self { config }
    }

    /// Build textured quads and triangle indices from a render plan.
    pub fn plan(&self, plan: &RenderPlan) -> std::result::Result<GlyphQuadBatch, GlyphQuadError> {
        self.validate_config()?;
        let mut quads = Vec::new();
        quads
            .try_reserve_exact(plan.glyphs.len())
            .map_err(|_| GlyphQuadError::IndexCountTooLarge)?;
        let mut indices = Vec::new();
        indices
            .try_reserve_exact(checked_glyph_quad_index_capacity(plan.glyphs.len())?)
            .map_err(|_| GlyphQuadError::IndexCountTooLarge)?;

        for glyph in &plan.glyphs {
            let quad =
                self.plan_glyph(glyph, plan.default_foreground_rgb8, plan.ansi_colors_rgb8)?;
            let base = checked_glyph_quad_base_index(quads.len())?;
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            quads.push(quad);
        }

        Ok(GlyphQuadBatch { quads, indices })
    }

    fn validate_config(&self) -> std::result::Result<(), GlyphQuadError> {
        if self.config.cell_width_px == 0
            || self.config.cell_height_px == 0
            || self.config.atlas_slot_width_px == 0
            || self.config.atlas_slot_height_px == 0
            || self.config.atlas_columns == 0
            || self.config.atlas_width_px == 0
            || self.config.atlas_height_px == 0
        {
            return Err(GlyphQuadError::ZeroDimension);
        }
        Ok(())
    }

    fn plan_glyph(
        &self,
        glyph: &PlannedGlyph,
        default_foreground_rgb8: [u8; 3],
        ansi_colors_rgb8: [[u8; 3]; 16],
    ) -> std::result::Result<GlyphQuad, GlyphQuadError> {
        let cell_width = self.config.cell_width_px as f32;
        let cell_height = self.config.cell_height_px as f32;
        let x0 = f32::from(glyph.col) * cell_width;
        let y0 = f32::from(glyph.row) * cell_height;
        let glyph_cells = if glyph.is_wide { 2.0 } else { 1.0 };
        let x1 = x0 + (cell_width * glyph_cells);
        let y1 = y0 + cell_height;

        let slot = glyph.atlas_entry.slot;
        let slot_col = slot % self.config.atlas_columns;
        let slot_row = slot / self.config.atlas_columns;
        let atlas_x0 = slot_col
            .checked_mul(self.config.atlas_slot_width_px)
            .ok_or(GlyphQuadError::SlotOutsideAtlas { slot })?;
        let atlas_y0 = slot_row
            .checked_mul(self.config.atlas_slot_height_px)
            .ok_or(GlyphQuadError::SlotOutsideAtlas { slot })?;
        let atlas_x1 = atlas_x0
            .checked_add(self.config.atlas_slot_width_px)
            .ok_or(GlyphQuadError::SlotOutsideAtlas { slot })?;
        let atlas_y1 = atlas_y0
            .checked_add(self.config.atlas_slot_height_px)
            .ok_or(GlyphQuadError::SlotOutsideAtlas { slot })?;
        if atlas_x1 > self.config.atlas_width_px || atlas_y1 > self.config.atlas_height_px {
            return Err(GlyphQuadError::SlotOutsideAtlas { slot });
        }

        let u0 = atlas_x0 as f32 / self.config.atlas_width_px as f32;
        let v0 = atlas_y0 as f32 / self.config.atlas_height_px as f32;
        let u1 = atlas_x1 as f32 / self.config.atlas_width_px as f32;
        let v1 = atlas_y1 as f32 / self.config.atlas_height_px as f32;
        let foreground_rgba =
            style_foreground_rgba(glyph.style, default_foreground_rgb8, ansi_colors_rgb8);

        Ok(GlyphQuad {
            text: glyph.text.clone(),
            ch: glyph.ch,
            atlas_entry: glyph.atlas_entry,
            vertices: [
                GlyphVertex {
                    position: [x0, y0],
                    uv: [u0, v0],
                    foreground_rgba,
                },
                GlyphVertex {
                    position: [x1, y0],
                    uv: [u1, v0],
                    foreground_rgba,
                },
                GlyphVertex {
                    position: [x1, y1],
                    uv: [u1, v1],
                    foreground_rgba,
                },
                GlyphVertex {
                    position: [x0, y1],
                    uv: [u0, v1],
                    foreground_rgba,
                },
            ],
        })
    }
}

fn checked_glyph_quad_base_index(quad_index: usize) -> std::result::Result<u32, GlyphQuadError> {
    u32::try_from(quad_index)
        .ok()
        .and_then(|index| index.checked_mul(4))
        .ok_or(GlyphQuadError::IndexCountTooLarge)
}

fn checked_glyph_quad_index_capacity(
    quad_count: usize,
) -> std::result::Result<usize, GlyphQuadError> {
    quad_count
        .checked_mul(6)
        .ok_or(GlyphQuadError::IndexCountTooLarge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_quad_base_index_accepts_last_representable_quad() {
        let last_valid_quad = usize::try_from(u32::MAX / 4).unwrap();

        assert_eq!(
            checked_glyph_quad_base_index(last_valid_quad).unwrap(),
            u32::MAX - 3
        );
    }

    #[test]
    fn glyph_quad_base_index_rejects_overflowing_quad_count() {
        let first_invalid_quad = usize::try_from(u32::MAX / 4).unwrap() + 1;

        let error = checked_glyph_quad_base_index(first_invalid_quad).unwrap_err();

        assert_eq!(error, GlyphQuadError::IndexCountTooLarge);
    }

    #[test]
    fn glyph_quad_index_capacity_uses_checked_multiplication() {
        assert_eq!(checked_glyph_quad_index_capacity(7).unwrap(), 42);

        let error = checked_glyph_quad_index_capacity((usize::MAX / 6) + 1).unwrap_err();

        assert_eq!(error, GlyphQuadError::IndexCountTooLarge);
    }
}
