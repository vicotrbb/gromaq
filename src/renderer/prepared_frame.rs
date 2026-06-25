//! Owned terminal glyph-frame preparation before native surface presentation.

mod construction;

use super::{BackgroundQuadBatch, GlyphAtlasImage, GlyphQuadBatch, SurfaceFrameError};
use crate::renderer::prepared_frame_preview::{PreparedFramePreview, preview_surface_glyph_frame};

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

/// Visual metrics and colors used to prepare a surface glyph frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedSurfaceGlyphFrameConfig {
    /// Terminal column width in pixels.
    pub cell_width_px: u16,
    /// Terminal row height in pixels.
    pub line_height_px: u16,
    /// Clear color used before drawing glyphs.
    pub clear_color: [f64; 4],
    /// Cursor color in RGBA8.
    pub cursor_color_rgba8: [u8; 4],
    /// Empty space around rendered terminal cells in physical pixels.
    pub surface_padding_px: u16,
    /// Visual gap between adjacent rendered terminal cells in physical pixels.
    pub cell_spacing_px: u16,
}

impl PreparedSurfaceGlyphFrame {
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

    /// Render this prepared glyph frame into a deterministic CPU-side RGBA8 preview.
    pub fn preview_rgba8(&self) -> std::result::Result<PreparedFramePreview, SurfaceFrameError> {
        preview_surface_glyph_frame(self.as_surface_glyph_frame())
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
