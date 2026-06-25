use super::rgba::{
    checked_rgba_row_offset, rgba_byte_len, rgba_pixel_count, rgba_row_byte_len, zeroed_rgba_buffer,
};
use super::{GlyphBitmap, GlyphImageError};
use crate::renderer::atlas::GlyphEntry;

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
