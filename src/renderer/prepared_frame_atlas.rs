//! Atlas helpers for owned surface glyph-frame preparation.

use super::{GlyphAtlasImage, GlyphBitmap, SurfaceFrameError};

pub(super) fn transparent_glyph_atlas(
    width: u32,
    height: u32,
) -> std::result::Result<GlyphAtlasImage, SurfaceFrameError> {
    let len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or_else(|| {
            SurfaceFrameError::InvalidFrame("transparent glyph atlas is too large".to_owned())
        })?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(len).map_err(|_| {
        SurfaceFrameError::InvalidFrame(
            "transparent glyph atlas is too large to allocate".to_owned(),
        )
    })?;
    rgba.resize(len, 0);
    Ok(GlyphAtlasImage {
        width,
        height,
        rgba,
        occupied_slots: 0,
    })
}

pub(super) fn atlas_columns_for_glyphs(glyphs: &[GlyphBitmap]) -> u32 {
    let slots = glyphs
        .iter()
        .map(|glyph| u64::from(glyph.entry.slot))
        .max()
        .unwrap_or(0)
        + 1;
    let mut columns = 1_u32;
    while u64::from(columns) * u64::from(columns) < slots {
        columns += 1;
    }
    columns
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::GlyphEntry;

    #[test]
    fn atlas_columns_for_glyphs_uses_widened_slot_math() {
        let glyphs = [
            GlyphBitmap {
                entry: GlyphEntry {
                    slot: 0,
                    generation: 0,
                },
                origin_x: 0,
                origin_y: 0,
                width: 1,
                height: 1,
                rgba: Vec::new(),
            },
            GlyphBitmap {
                entry: GlyphEntry {
                    slot: 3,
                    generation: 0,
                },
                origin_x: 0,
                origin_y: 0,
                width: 1,
                height: 1,
                rgba: Vec::new(),
            },
        ];

        assert_eq!(atlas_columns_for_glyphs(&glyphs), 2);
    }

    #[test]
    fn atlas_columns_for_glyphs_handles_maximum_slot_without_overflow() {
        let glyphs = [GlyphBitmap {
            entry: GlyphEntry {
                slot: u32::MAX,
                generation: 0,
            },
            origin_x: 0,
            origin_y: 0,
            width: 1,
            height: 1,
            rgba: Vec::new(),
        }];

        assert_eq!(atlas_columns_for_glyphs(&glyphs), 65_536);
    }
}
