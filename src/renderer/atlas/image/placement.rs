use super::rgba::{checked_rgba_row_offset, rgba_byte_len, rgba_row_byte_len, zeroed_rgba_buffer};
use super::{GlyphBitmap, GlyphImageError};

impl GlyphBitmap {
    /// Return this glyph copied into a larger transparent bitmap using shaped cell placement.
    pub fn padded_to_terminal_slot(
        &self,
        target_width: u32,
        target_height: u32,
    ) -> std::result::Result<Self, GlyphImageError> {
        let placement = self.terminal_slot_placement(target_width, target_height)?;
        self.padded_to_slot_at(target_width, target_height, placement.x, placement.y)
    }

    /// Minimum fixed atlas slot width needed to preserve this glyph's horizontal placement.
    pub fn terminal_slot_width(&self, cell_width: u32) -> u32 {
        let left = self.origin_x.saturating_neg().max(0) as u32;
        let right = self.origin_x.saturating_add_unsigned(self.width).max(0) as u32;
        left.saturating_add(right).max(cell_width).max(self.width)
    }

    /// Minimum fixed atlas slot height needed to preserve this glyph's vertical placement.
    pub fn terminal_slot_height(&self, cell_height: u32) -> u32 {
        let above_baseline = self.origin_y.saturating_neg().max(0) as u32;
        let below_baseline = self.origin_y.saturating_add_unsigned(self.height).max(0) as u32;
        above_baseline
            .saturating_add(below_baseline)
            .max(cell_height)
            .max(self.height)
    }

    fn terminal_slot_placement(
        &self,
        target_width: u32,
        target_height: u32,
    ) -> std::result::Result<TerminalSlotPlacement, GlyphImageError> {
        let required_width = self.terminal_slot_width(0);
        let required_height = self.terminal_slot_height(0);
        if target_width < required_width || target_height < required_height {
            return Err(GlyphImageError::PaddingTargetTooSmall {
                target_width,
                target_height,
                glyph_width: required_width,
                glyph_height: required_height,
            });
        }

        let left_bearing = self.origin_x.saturating_neg().max(0) as u32;
        let above_baseline = self.origin_y.saturating_neg().max(0) as u32;
        let extra_x = (target_width - required_width) / 2;
        let extra_y = (target_height - required_height) / 2;
        let x = i32::try_from(left_bearing.saturating_add(extra_x))
            .ok()
            .and_then(|origin| origin.checked_add(self.origin_x))
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(GlyphImageError::RgbaImageOffsetTooLarge)?;
        let y = i32::try_from(above_baseline.saturating_add(extra_y))
            .ok()
            .and_then(|baseline| baseline.checked_add(self.origin_y))
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(GlyphImageError::RgbaImageOffsetTooLarge)?;

        Ok(TerminalSlotPlacement { x, y })
    }

    fn padded_to_slot_at(
        &self,
        target_width: u32,
        target_height: u32,
        target_x: u32,
        target_y: u32,
    ) -> std::result::Result<Self, GlyphImageError> {
        if target_width < self.width || target_height < self.height {
            return Err(GlyphImageError::PaddingTargetTooSmall {
                target_width,
                target_height,
                glyph_width: self.width,
                glyph_height: self.height,
            });
        }
        if target_x
            .checked_add(self.width)
            .is_none_or(|right| right > target_width)
            || target_y
                .checked_add(self.height)
                .is_none_or(|bottom| bottom > target_height)
        {
            return Err(GlyphImageError::PaddingTargetTooSmall {
                target_width,
                target_height,
                glyph_width: self.width,
                glyph_height: self.height,
            });
        }
        if self.width == target_width
            && self.height == target_height
            && target_x == 0
            && target_y == 0
        {
            let mut glyph = self.clone();
            glyph.origin_x = 0;
            glyph.origin_y = 0;
            return Ok(glyph);
        }

        let source_row_bytes = rgba_row_byte_len(self.width)?;
        let target_row_bytes = rgba_row_byte_len(target_width)?;
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
        let target_y =
            usize::try_from(target_y).map_err(|_| GlyphImageError::RgbaImageDimensionsTooLarge)?;
        let mut rgba = zeroed_rgba_buffer(target_width, target_height)?;
        for row in 0..source_height {
            let source_start = checked_rgba_row_offset(row, source_row_bytes)?;
            let target_row = row
                .checked_add(target_y)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerminalSlotPlacement {
    x: u32,
    y: u32,
}
