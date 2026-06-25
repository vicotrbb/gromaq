use thiserror::Error;

use crate::renderer::atlas::GlyphEntry;

mod indices;
mod planner;

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
    pub(super) config: GlyphQuadConfig,
}
