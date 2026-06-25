use thiserror::Error;

use super::GlyphEntry;
use rgba::{
    checked_rgba_row_offset, rgba_byte_len, rgba_pixel_count, rgba_row_byte_len, zeroed_rgba_buffer,
};

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

impl GlyphBitmap {
    /// Try to build a solid RGBA8 glyph bitmap without panicking on oversized dimensions.
    pub fn try_solid_rgba8(
        entry: GlyphEntry,
        width: u32,
        height: u32,
        rgba: [u8; 4],
    ) -> std::result::Result<Self, GlyphImageError> {
        let pixel_count = rgba_pixel_count(width, height)?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(rgba_byte_len(width, height)?)
            .map_err(|_| GlyphImageError::SolidGlyphAllocationTooLarge)?;
        for _ in 0..pixel_count {
            pixels.extend_from_slice(&rgba);
        }
        Ok(Self {
            entry,
            origin_x: 0,
            origin_y: 0,
            width,
            height,
            rgba: pixels,
        })
    }

    /// Return this glyph copied into the optical center of a larger transparent bitmap.
    pub fn padded_to(
        &self,
        target_width: u32,
        target_height: u32,
    ) -> std::result::Result<Self, GlyphImageError> {
        if target_width < self.width || target_height < self.height {
            return Err(GlyphImageError::PaddingTargetTooSmall {
                target_width,
                target_height,
                glyph_width: self.width,
                glyph_height: self.height,
            });
        }
        if self.width == target_width && self.height == target_height {
            return Ok(self.clone());
        }

        let source_row_bytes = rgba_row_byte_len(self.width)?;
        let target_row_bytes = rgba_row_byte_len(target_width)?;
        let target_x = (target_width - self.width) / 2;
        let target_y = (target_height - self.height) / 2;
        let target_x_bytes = rgba_row_byte_len(target_x)?;
        let expected_source_len = rgba_byte_len(self.width, self.height)?;
        if self.rgba.len() != expected_source_len {
            return Err(GlyphImageError::InvalidPaddingSourceLength {
                slot: self.entry.slot,
                expected_len: expected_source_len,
            });
        }

        let source_height = usize::try_from(self.height)
            .map_err(|_| GlyphImageError::RgbaImageDimensionsTooLarge)?;
        let mut rgba = zeroed_rgba_buffer(target_width, target_height)?;
        for row in 0..source_height {
            let source_start = checked_rgba_row_offset(row, source_row_bytes)?;
            let target_row = row
                .checked_add(
                    usize::try_from(target_y)
                        .map_err(|_| GlyphImageError::RgbaImageDimensionsTooLarge)?,
                )
                .ok_or(GlyphImageError::RgbaRowOffsetTooLarge)?;
            let target_start = checked_rgba_row_offset(target_row, target_row_bytes)?
                .checked_add(target_x_bytes)
                .ok_or(GlyphImageError::RgbaRowOffsetTooLarge)?;
            rgba[target_start..target_start + source_row_bytes]
                .copy_from_slice(&self.rgba[source_start..source_start + source_row_bytes]);
        }

        Ok(Self {
            entry: self.entry,
            origin_x: 0,
            origin_y: 0,
            width: target_width,
            height: target_height,
            rgba,
        })
    }
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
