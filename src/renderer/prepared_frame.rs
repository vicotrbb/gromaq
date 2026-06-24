//! Owned terminal glyph-frame preparation before native surface presentation.

use super::{
    BackgroundQuadBatch, BackgroundQuadConfig, BackgroundQuadPlanner, CursorQuadConfig,
    CursorQuadPlanner, GlyphAtlasImage, GlyphBitmap, GlyphQuadBatch, GlyphQuadConfig,
    GlyphQuadPlanner, RenderPlan, SurfaceFrameError, TextDecorationQuadConfig,
    TextDecorationQuadPlanner,
};
use crate::renderer::prepared_frame_atlas::{atlas_columns_for_glyphs, transparent_glyph_atlas};
use crate::renderer::prepared_frame_geometry::{
    checked_surface_frame_pixel_dimension, translate_background_batch, translate_glyph_batch,
};

/// Glyph frame data ready for presentation to a native surface.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceGlyphFrame<'a> {
    /// Packed glyph atlas image sampled by the frame.
    pub atlas: &'a GlyphAtlasImage,
    /// Solid background quads drawn before textured glyphs.
    pub background_batch: &'a BackgroundQuadBatch,
    /// Textured glyph quads and indices to draw.
    pub batch: &'a GlyphQuadBatch,
    /// Solid text-decoration quads drawn after textured glyphs.
    pub decoration_batch: &'a BackgroundQuadBatch,
    /// Solid cursor quads drawn after textured glyphs.
    pub cursor_batch: &'a BackgroundQuadBatch,
    /// Surface frame width in pixels.
    pub width: u32,
    /// Surface frame height in pixels.
    pub height: u32,
    /// Clear color used before drawing glyphs.
    pub clear_color: [f64; 4],
}

/// Owned terminal glyph frame prepared for presentation to a native surface.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedSurfaceGlyphFrame {
    atlas: GlyphAtlasImage,
    background_batch: BackgroundQuadBatch,
    batch: GlyphQuadBatch,
    decoration_batch: BackgroundQuadBatch,
    cursor_batch: BackgroundQuadBatch,
    width: u32,
    height: u32,
    clear_color: [f64; 4],
}

impl PreparedSurfaceGlyphFrame {
    /// Build an owned presentable glyph frame from a render plan and rasterized glyph bitmaps.
    pub fn from_render_plan(
        plan: &RenderPlan,
        glyphs: &[GlyphBitmap],
        cell_width_px: u16,
        line_height_px: u16,
        clear_color: [f64; 4],
        cursor_color_rgba8: [u8; 4],
        surface_padding_px: u16,
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

        let cell_width_px = u32::from(cell_width_px);
        let cell_height_px = u32::from(line_height_px);
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
            surface_padding_px,
        )?;
        let height = checked_surface_frame_pixel_dimension(
            "surface glyph frame height",
            plan.viewport_rows,
            cell_height_px,
            surface_padding_px,
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
            color_rgba8: cursor_color_rgba8,
        })
        .plan(plan)
        .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?;
        translate_glyph_batch(&mut batch, surface_padding_px);
        translate_background_batch(&mut background_batch, surface_padding_px);
        translate_background_batch(&mut decoration_batch, surface_padding_px);
        translate_background_batch(&mut cursor_batch, surface_padding_px);
        Ok(Self {
            atlas,
            background_batch,
            batch,
            decoration_batch,
            cursor_batch,
            width,
            height,
            clear_color,
        })
    }

    /// Borrow this owned frame as a surface presentation frame.
    pub fn as_surface_glyph_frame(&self) -> SurfaceGlyphFrame<'_> {
        SurfaceGlyphFrame {
            atlas: &self.atlas,
            background_batch: &self.background_batch,
            batch: &self.batch,
            decoration_batch: &self.decoration_batch,
            cursor_batch: &self.cursor_batch,
            width: self.width,
            height: self.height,
            clear_color: self.clear_color,
        }
    }

    /// Packed atlas image for this frame.
    pub fn atlas(&self) -> &GlyphAtlasImage {
        &self.atlas
    }

    /// Glyph quad batch for this frame.
    pub fn batch(&self) -> &GlyphQuadBatch {
        &self.batch
    }

    /// Solid background quad batch for this frame.
    pub fn background_batch(&self) -> &BackgroundQuadBatch {
        &self.background_batch
    }

    /// Solid text-decoration quad batch for this frame.
    pub fn decoration_batch(&self) -> &BackgroundQuadBatch {
        &self.decoration_batch
    }

    /// Solid cursor quad batch for this frame.
    pub fn cursor_batch(&self) -> &BackgroundQuadBatch {
        &self.cursor_batch
    }
}
