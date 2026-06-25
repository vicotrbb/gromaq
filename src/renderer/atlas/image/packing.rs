use super::rgba::{rgba_byte_len, rgba_offset, rgba_row_byte_len, zeroed_rgba_buffer};
use super::{GlyphAtlasImage, GlyphBitmap, GlyphImageError};
use crate::renderer::GlyphEntry;

impl GlyphAtlasImage {
    /// Pack fixed-size RGBA8 glyph bitmaps into slots.
    pub fn pack_rgba8(
        slot_width: u32,
        slot_height: u32,
        columns: u32,
        glyphs: &[GlyphBitmap],
    ) -> std::result::Result<Self, GlyphImageError> {
        if slot_width == 0 || slot_height == 0 || columns == 0 {
            return Err(GlyphImageError::InvalidAtlasSlotLayout);
        }
        let max_slot = glyphs
            .iter()
            .map(|glyph| glyph.entry.slot)
            .max()
            .unwrap_or(0);
        let rows = (max_slot / columns) + 1;
        let width = slot_width
            .checked_mul(columns)
            .ok_or(GlyphImageError::AtlasWidthTooLarge)?;
        let height = slot_height
            .checked_mul(rows)
            .ok_or(GlyphImageError::AtlasHeightTooLarge)?;
        let mut rgba = zeroed_rgba_buffer(width, height)?;

        for glyph in glyphs {
            let expected_len = rgba_byte_len(slot_width, slot_height)?;
            if glyph.width != slot_width
                || glyph.height != slot_height
                || glyph.rgba.len() != expected_len
            {
                return Err(GlyphImageError::InvalidAtlasGlyphSize {
                    slot: glyph.entry.slot,
                    expected_len,
                    slot_width,
                    slot_height,
                });
            }

            copy_glyph_to_atlas_slot(&mut rgba, width, slot_width, slot_height, columns, glyph)?;
        }

        Ok(Self {
            width,
            height,
            rgba,
            occupied_slots: glyphs.len(),
        })
    }

    /// Build a deterministic two-slot atlas image for GPU upload smoke tests.
    pub fn smoke_rgba8() -> std::result::Result<Self, GlyphImageError> {
        let red = GlyphBitmap::try_solid_rgba8(
            GlyphEntry {
                slot: 0,
                generation: 0,
            },
            2,
            2,
            [255, 0, 0, 255],
        )?;
        let green = GlyphBitmap::try_solid_rgba8(
            GlyphEntry {
                slot: 1,
                generation: 0,
            },
            2,
            2,
            [0, 255, 0, 255],
        )?;
        Self::pack_rgba8(2, 2, 2, &[red, green])
    }
}

fn copy_glyph_to_atlas_slot(
    atlas_rgba: &mut [u8],
    atlas_width: u32,
    slot_width: u32,
    slot_height: u32,
    columns: u32,
    glyph: &GlyphBitmap,
) -> std::result::Result<(), GlyphImageError> {
    let slot_col = glyph.entry.slot % columns;
    let slot_row = glyph.entry.slot / columns;
    for y in 0..slot_height {
        let atlas_y = slot_row
            .checked_mul(slot_height)
            .and_then(|row_start| row_start.checked_add(y))
            .ok_or(GlyphImageError::AtlasRowOffsetTooLarge)?;
        let atlas_x = slot_col
            .checked_mul(slot_width)
            .ok_or(GlyphImageError::AtlasColumnOffsetTooLarge)?;
        let atlas_start = rgba_offset(atlas_width, atlas_x, atlas_y)?;
        let glyph_start = rgba_offset(slot_width, 0, y)?;
        let row_bytes = rgba_row_byte_len(slot_width)?;
        atlas_rgba[atlas_start..atlas_start + row_bytes]
            .copy_from_slice(&glyph.rgba[glyph_start..glyph_start + row_bytes]);
    }
    Ok(())
}
