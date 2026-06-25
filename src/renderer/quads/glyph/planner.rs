use crate::renderer::color::style_foreground_rgba;
use crate::renderer::{PlannedGlyph, RenderPlan};

use super::indices::{checked_glyph_quad_base_index, checked_glyph_quad_index_capacity};
use super::{
    GlyphQuad, GlyphQuadBatch, GlyphQuadConfig, GlyphQuadError, GlyphQuadPlanner, GlyphVertex,
};

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
            let quad = self.plan_glyph(
                glyph,
                plan.default_foreground_rgb8,
                plan.ansi_colors_rgb8,
                plan.dim_opacity,
            )?;
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
        dim_opacity: f32,
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
        let foreground_rgba = style_foreground_rgba(
            glyph.style,
            default_foreground_rgb8,
            ansi_colors_rgb8,
            dim_opacity,
        );

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
