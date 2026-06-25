use super::super::{
    BackgroundQuadConfig, BackgroundQuadPlanner, CursorQuadConfig, CursorQuadPlanner,
    GlyphAtlasImage, GlyphBitmap, GlyphQuadConfig, GlyphQuadPlanner, RenderPlan, SurfaceFrameError,
    TextDecorationQuadConfig, TextDecorationQuadPlanner,
};
use super::{PreparedSurfaceGlyphFrame, PreparedSurfaceGlyphFrameConfig};
use crate::renderer::prepared_frame_atlas::{atlas_columns_for_glyphs, transparent_glyph_atlas};
use crate::renderer::prepared_frame_geometry::{
    apply_background_cell_spacing, apply_glyph_cell_spacing, checked_surface_frame_pixel_dimension,
    translate_background_batch, translate_glyph_batch,
};

impl PreparedSurfaceGlyphFrame {
    /// Build an owned presentable glyph frame from a render plan and rasterized glyph bitmaps.
    pub fn from_render_plan(
        plan: &RenderPlan,
        glyphs: &[GlyphBitmap],
        config: PreparedSurfaceGlyphFrameConfig,
    ) -> std::result::Result<Self, SurfaceFrameError> {
        if !plan.glyphs.is_empty() && glyphs.is_empty() {
            return Err(SurfaceFrameError::InvalidFrame(
                "surface glyph frame requires rasterized glyph bitmaps".to_owned(),
            ));
        }
        for planned in &plan.glyphs {
            if !glyphs
                .iter()
                .any(|glyph| glyph.entry == planned.atlas_entry)
            {
                return Err(SurfaceFrameError::InvalidFrame(format!(
                    "missing rasterized bitmap for atlas slot {} generation {}",
                    planned.atlas_entry.slot, planned.atlas_entry.generation
                )));
            }
        }

        let cell_width_px = u32::from(config.cell_width_px);
        let cell_height_px = u32::from(config.line_height_px);
        let slot_width = glyphs
            .iter()
            .map(|glyph| glyph.terminal_slot_width(cell_width_px))
            .max()
            .unwrap_or(cell_width_px)
            .max(cell_width_px);
        let slot_height = glyphs
            .iter()
            .map(|glyph| glyph.terminal_slot_height(cell_height_px))
            .max()
            .unwrap_or(cell_height_px)
            .max(cell_height_px);
        if slot_width == 0 || slot_height == 0 {
            return Err(SurfaceFrameError::InvalidFrame(
                "surface frame cell dimensions must be non-zero".to_owned(),
            ));
        }
        let width = checked_surface_frame_pixel_dimension(
            "surface glyph frame width",
            plan.viewport_cols,
            cell_width_px,
            config.surface_padding_px,
            config.cell_spacing_px,
        )?;
        let height = checked_surface_frame_pixel_dimension(
            "surface glyph frame height",
            plan.viewport_rows,
            cell_height_px,
            config.surface_padding_px,
            config.cell_spacing_px,
        )?;
        let padded = glyphs
            .iter()
            .map(|glyph| {
                glyph
                    .padded_to_terminal_slot(slot_width, slot_height)
                    .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let (columns, atlas) = if padded.is_empty() {
            (
                1,
                transparent_glyph_atlas(slot_width, slot_height)
                    .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?,
            )
        } else {
            let columns = atlas_columns_for_glyphs(&padded);
            let atlas = GlyphAtlasImage::pack_rgba8(slot_width, slot_height, columns, &padded)
                .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?;
            (columns, atlas)
        };
        let mut batch = GlyphQuadPlanner::new(GlyphQuadConfig {
            cell_width_px,
            cell_height_px,
            atlas_slot_width_px: slot_width,
            atlas_slot_height_px: slot_height,
            atlas_columns: columns,
            atlas_width_px: atlas.width,
            atlas_height_px: atlas.height,
        })
        .plan(plan)
        .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?;
        let mut background_batch = BackgroundQuadPlanner::new(BackgroundQuadConfig {
            cell_width_px,
            cell_height_px,
        })
        .plan(plan)
        .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?;
        let mut decoration_batch = TextDecorationQuadPlanner::new(TextDecorationQuadConfig {
            cell_width_px,
            cell_height_px,
        })
        .plan(plan)
        .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?;
        let mut cursor_batch = CursorQuadPlanner::new(CursorQuadConfig {
            cell_width_px,
            cell_height_px,
            color_rgba8: config.cursor_color_rgba8,
        })
        .plan(plan)
        .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?;
        apply_glyph_cell_spacing(&mut batch, cell_width_px, config.cell_spacing_px);
        apply_background_cell_spacing(&mut background_batch, config.cell_spacing_px);
        apply_background_cell_spacing(&mut decoration_batch, config.cell_spacing_px);
        apply_background_cell_spacing(&mut cursor_batch, config.cell_spacing_px);
        translate_glyph_batch(&mut batch, config.surface_padding_px);
        translate_background_batch(&mut background_batch, config.surface_padding_px);
        translate_background_batch(&mut decoration_batch, config.surface_padding_px);
        translate_background_batch(&mut cursor_batch, config.surface_padding_px);
        Ok(Self {
            atlas,
            background_batch,
            batch,
            decoration_batch,
            cursor_batch,
            width,
            height,
            clear_color: config.clear_color,
        })
    }
}
