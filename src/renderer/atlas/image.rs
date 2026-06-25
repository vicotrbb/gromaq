use thiserror::Error;

use super::GlyphEntry;

mod bitmap;
mod packing;
mod placement;
mod rgba;

/// One rasterized glyph bitmap ready for atlas packing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphBitmap {
    /// Atlas entry this bitmap belongs to.
    pub entry: GlyphEntry,
    /// Left edge of the glyph bitmap relative to the shaped cell origin.
    pub origin_x: i32,
    /// Top edge of the glyph bitmap relative to the shaped baseline.
    pub origin_y: i32,
    /// Bitmap width in pixels.
    pub width: u32,
    /// Bitmap height in pixels.
    pub height: u32,
    /// Dense RGBA8 pixels in row-major order.
    pub rgba: Vec<u8>,
}

/// Errors produced while building or packing dense RGBA8 glyph images.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GlyphImageError {
    /// A solid glyph bitmap could not reserve its pixel buffer.
    #[error("solid glyph bitmap is too large to allocate")]
    SolidGlyphAllocationTooLarge,
    /// A requested RGBA row cannot be represented in memory.
    #[error("rgba row dimensions are too large")]
    RgbaRowDimensionsTooLarge,
    /// A requested RGBA image cannot be represented in memory.
    #[error("rgba image dimensions are too large")]
    RgbaImageDimensionsTooLarge,
    /// A requested RGBA row offset cannot be represented in memory.
    #[error("rgba row offset is too large")]
    RgbaRowOffsetTooLarge,
    /// A requested RGBA image buffer could not be allocated.
    #[error("rgba image buffer is too large to allocate")]
    RgbaBufferAllocationTooLarge,
    /// A requested RGBA pixel offset cannot be represented in memory.
    #[error("rgba image offset is too large")]
    RgbaImageOffsetTooLarge,
    /// The padding target is smaller than the source glyph bitmap.
    #[error(
        "target {target_width}x{target_height} is smaller than glyph {glyph_width}x{glyph_height}"
    )]
    PaddingTargetTooSmall {
        /// Requested padded bitmap width.
        target_width: u32,
        /// Requested padded bitmap height.
        target_height: u32,
        /// Source glyph bitmap width.
        glyph_width: u32,
        /// Source glyph bitmap height.
        glyph_height: u32,
    },
    /// A source glyph bitmap does not contain the expected dense RGBA8 byte length.
    #[error("glyph slot {slot} expected {expected_len} rgba bytes before padding")]
    InvalidPaddingSourceLength {
        /// Atlas slot for the malformed glyph.
        slot: u32,
        /// Expected dense RGBA8 byte length.
        expected_len: usize,
    },
    /// Fixed-size atlas slot dimensions must be non-zero.
    #[error("slot dimensions and columns must be non-zero")]
    InvalidAtlasSlotLayout,
    /// The packed atlas width cannot be represented.
    #[error("glyph atlas width is too large")]
    AtlasWidthTooLarge,
    /// The packed atlas height cannot be represented.
    #[error("glyph atlas height is too large")]
    AtlasHeightTooLarge,
    /// A glyph bitmap does not match the requested fixed atlas slot size.
    #[error("glyph slot {slot} expected {expected_len} rgba bytes for {slot_width}x{slot_height}")]
    InvalidAtlasGlyphSize {
        /// Atlas slot for the malformed glyph.
        slot: u32,
        /// Expected dense RGBA8 byte length.
        expected_len: usize,
        /// Expected slot width.
        slot_width: u32,
        /// Expected slot height.
        slot_height: u32,
    },
    /// The packed atlas row offset cannot be represented.
    #[error("glyph atlas row offset is too large")]
    AtlasRowOffsetTooLarge,
    /// The packed atlas column offset cannot be represented.
    #[error("glyph atlas column offset is too large")]
    AtlasColumnOffsetTooLarge,
}

/// Packed RGBA8 glyph atlas image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphAtlasImage {
    /// Atlas image width in pixels.
    pub width: u32,
    /// Atlas image height in pixels.
    pub height: u32,
    /// Dense RGBA8 pixels in row-major order.
    pub rgba: Vec<u8>,
    /// Number of populated atlas slots.
    pub occupied_slots: usize,
}
