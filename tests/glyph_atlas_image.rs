use gromaq::renderer::{GlyphAtlasImage, GlyphBitmap, GlyphEntry, GlyphImageError};

#[test]
fn glyph_atlas_image_packs_bitmaps_by_slot() {
    let red = GlyphBitmap::try_solid_rgba8(
        GlyphEntry {
            slot: 0,
            generation: 0,
        },
        2,
        2,
        [255, 0, 0, 255],
    )
    .unwrap();
    let green = GlyphBitmap::try_solid_rgba8(
        GlyphEntry {
            slot: 1,
            generation: 0,
        },
        2,
        2,
        [0, 255, 0, 255],
    )
    .unwrap();

    let image = GlyphAtlasImage::pack_rgba8(2, 2, 2, &[red, green]).unwrap();

    assert_eq!(image.width, 4);
    assert_eq!(image.height, 2);
    assert_eq!(&image.rgba[0..4], &[255, 0, 0, 255]);
    assert_eq!(&image.rgba[8..12], &[0, 255, 0, 255]);
    assert_eq!(image.occupied_slots, 2);
}

#[test]
fn glyph_atlas_image_rejects_wrong_bitmap_size() {
    let bad = GlyphBitmap {
        entry: GlyphEntry {
            slot: 0,
            generation: 0,
        },
        origin_x: 0,
        origin_y: 0,
        width: 2,
        height: 2,
        rgba: vec![255; 4],
    };

    let error = GlyphAtlasImage::pack_rgba8(2, 2, 1, &[bad]).unwrap_err();

    assert_eq!(
        error,
        GlyphImageError::InvalidAtlasGlyphSize {
            slot: 0,
            expected_len: 16,
            slot_width: 2,
            slot_height: 2
        }
    );
}

#[test]
fn glyph_bitmap_padding_centers_smaller_glyph_in_target_slot() {
    let glyph = GlyphBitmap::try_solid_rgba8(
        GlyphEntry {
            slot: 0,
            generation: 0,
        },
        1,
        1,
        [12, 34, 56, 255],
    )
    .unwrap();

    let padded = glyph.padded_to(3, 3).unwrap();

    assert_eq!(padded.width, 3);
    assert_eq!(padded.height, 3);
    assert_eq!(&padded.rgba[16..20], &[12, 34, 56, 255]);
    assert!(padded.rgba[0..16].iter().all(|byte| *byte == 0));
    assert!(padded.rgba[20..].iter().all(|byte| *byte == 0));
}

#[test]
fn glyph_bitmap_terminal_slot_padding_preserves_shaped_baseline_placement() {
    let glyph = GlyphBitmap {
        entry: GlyphEntry {
            slot: 0,
            generation: 0,
        },
        origin_x: 1,
        origin_y: -3,
        width: 2,
        height: 5,
        rgba: [12, 34, 56, 255].repeat(10),
    };

    let padded = glyph.padded_to_terminal_slot(6, 10).unwrap();

    assert_eq!(padded.origin_x, 0);
    assert_eq!(padded.origin_y, 0);
    assert_eq!(padded.width, 6);
    assert_eq!(padded.height, 10);
    assert_eq!(&padded.rgba[56..60], &[12, 34, 56, 255]);
    assert_eq!(&padded.rgba[60..64], &[12, 34, 56, 255]);
    assert!(padded.rgba[0..56].iter().all(|byte| *byte == 0));
}

#[test]
fn solid_glyph_bitmap_rejects_overflowing_dimensions_before_allocation() {
    let error = GlyphBitmap::try_solid_rgba8(
        GlyphEntry {
            slot: 0,
            generation: 0,
        },
        u32::MAX,
        u32::MAX,
        [255, 255, 255, 255],
    )
    .unwrap_err();

    assert_eq!(error, GlyphImageError::RgbaImageDimensionsTooLarge);
}

#[test]
fn glyph_atlas_image_rejects_overflowing_dimensions_before_allocation() {
    let width_error = GlyphAtlasImage::pack_rgba8(u32::MAX, 1, 2, &[]).unwrap_err();
    assert_eq!(width_error, GlyphImageError::AtlasWidthTooLarge);

    let tall_glyph = GlyphBitmap {
        entry: GlyphEntry {
            slot: 1,
            generation: 0,
        },
        origin_x: 0,
        origin_y: 0,
        width: 1,
        height: u32::MAX,
        rgba: Vec::new(),
    };
    let height_error = GlyphAtlasImage::pack_rgba8(1, u32::MAX, 1, &[tall_glyph]).unwrap_err();
    assert_eq!(height_error, GlyphImageError::AtlasHeightTooLarge);

    let huge_slot = GlyphBitmap {
        entry: GlyphEntry {
            slot: u32::MAX - 1,
            generation: 0,
        },
        origin_x: 0,
        origin_y: 0,
        width: u32::MAX,
        height: 1,
        rgba: Vec::new(),
    };
    let image_error = GlyphAtlasImage::pack_rgba8(u32::MAX, 1, 1, &[huge_slot]).unwrap_err();
    assert_eq!(image_error, GlyphImageError::RgbaImageDimensionsTooLarge);
}

#[test]
fn glyph_bitmap_padding_rejects_oversized_target_before_allocation() {
    let glyph = GlyphBitmap::try_solid_rgba8(
        GlyphEntry {
            slot: 0,
            generation: 0,
        },
        1,
        1,
        [255, 255, 255, 255],
    )
    .unwrap();

    let error = glyph.padded_to(u32::MAX, u32::MAX).unwrap_err();

    assert_eq!(error, GlyphImageError::RgbaImageDimensionsTooLarge);
}
