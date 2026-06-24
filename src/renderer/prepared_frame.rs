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
        fallback_cell_size_px: u16,
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

        let fallback_cell_size_px = u32::from(fallback_cell_size_px);
        let line_height_px = u32::from(line_height_px);
        let slot_width = glyphs
            .iter()
            .map(|glyph| glyph.width)
            .max()
            .unwrap_or(fallback_cell_size_px);
        let slot_height = glyphs
            .iter()
            .map(|glyph| glyph.height)
            .max()
            .unwrap_or(fallback_cell_size_px)
            .max(line_height_px);
        if slot_width == 0 || slot_height == 0 {
            return Err(SurfaceFrameError::InvalidFrame(
                "surface frame cell dimensions must be non-zero".to_owned(),
            ));
        }
        let width = checked_surface_frame_pixel_dimension(
            "surface glyph frame width",
            plan.viewport_cols,
            slot_width,
            surface_padding_px,
        )?;
        let height = checked_surface_frame_pixel_dimension(
            "surface glyph frame height",
            plan.viewport_rows,
            slot_height,
            surface_padding_px,
        )?;
        let padded = glyphs
            .iter()
            .map(|glyph| {
                glyph
                    .padded_to(slot_width, slot_height)
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
            cell_width_px: slot_width,
            cell_height_px: slot_height,
            atlas_slot_width_px: slot_width,
            atlas_slot_height_px: slot_height,
            atlas_columns: columns,
            atlas_width_px: atlas.width,
            atlas_height_px: atlas.height,
        })
        .plan(plan)
        .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?;
        let mut background_batch = BackgroundQuadPlanner::new(BackgroundQuadConfig {
            cell_width_px: slot_width,
            cell_height_px: slot_height,
        })
        .plan(plan)
        .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?;
        let mut decoration_batch = TextDecorationQuadPlanner::new(TextDecorationQuadConfig {
            cell_width_px: slot_width,
            cell_height_px: slot_height,
        })
        .plan(plan)
        .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?;
        let mut cursor_batch = CursorQuadPlanner::new(CursorQuadConfig {
            cell_width_px: slot_width,
            cell_height_px: slot_height,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::Style;
    use crate::config::DEFAULT_ANSI_COLORS_RGB8;
    use crate::renderer::{GlyphEntry, PlannedGlyph};
    use crate::terminal::{CursorShape, CursorSnapshot};

    #[test]
    fn prepared_surface_glyph_frame_rejects_overflowing_pixel_width() {
        let entry = GlyphEntry {
            slot: 0,
            generation: 0,
        };
        let plan = RenderPlan {
            viewport_cols: 2,
            viewport_rows: 1,
            cursor: CursorSnapshot {
                row: 0,
                col: 0,
                visible: true,
                shape: CursorShape::Block,
                blinking: true,
            },
            default_foreground_rgb8: [229, 229, 229],
            ansi_colors_rgb8: DEFAULT_ANSI_COLORS_RGB8,
            clear_regions: Vec::new(),
            backgrounds: Vec::new(),
            decorations: Vec::new(),
            glyphs: vec![PlannedGlyph {
                row: 0,
                col: 0,
                text: "A".to_owned(),
                ch: 'A',
                style: Style::default(),
                font_size_px: 14,
                is_wide: false,
                atlas_entry: entry,
            }],
        };
        let glyphs = [GlyphBitmap {
            entry,
            width: u32::MAX,
            height: 1,
            rgba: Vec::new(),
        }];

        let error = PreparedSurfaceGlyphFrame::from_render_plan(
            &plan,
            &glyphs,
            14,
            14,
            [0.0, 0.0, 0.0, 1.0],
            [244, 192, 106, 255],
            0,
        )
        .unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame(
                "surface glyph frame width is too large to represent".to_owned()
            )
        );
    }

    #[test]
    fn prepared_surface_glyph_frame_builds_cursor_only_blank_frame() {
        let plan = RenderPlan {
            viewport_cols: 8,
            viewport_rows: 2,
            cursor: CursorSnapshot {
                row: 0,
                col: 0,
                visible: true,
                shape: CursorShape::Block,
                blinking: true,
            },
            default_foreground_rgb8: [232, 226, 214],
            ansi_colors_rgb8: DEFAULT_ANSI_COLORS_RGB8,
            clear_regions: Vec::new(),
            backgrounds: Vec::new(),
            decorations: Vec::new(),
            glyphs: Vec::new(),
        };

        let prepared = PreparedSurfaceGlyphFrame::from_render_plan(
            &plan,
            &[],
            18,
            22,
            [0.0, 0.0, 0.0, 1.0],
            [244, 192, 106, 255],
            12,
        )
        .unwrap();
        let frame = prepared.as_surface_glyph_frame();
        assert_eq!(frame.height, (2 * 22) + (2 * 12));

        assert!(frame.batch.quads.is_empty());
        assert_eq!(frame.cursor_batch.quads.len(), 1);
        assert_eq!(frame.cursor_batch.indices.len(), 6);
        assert_eq!(frame.atlas.occupied_slots, 0);
        assert_eq!(frame.atlas.width, 18);
        assert_eq!(frame.atlas.height, 22);
        assert_eq!(frame.width, 168);
        assert!(frame.atlas.rgba.iter().all(|byte| *byte == 0));
    }
}
