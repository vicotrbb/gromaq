use gromaq::renderer::{GlyphAtlasImage, GlyphBitmap, GlyphEntry};

#[test]
fn glyph_atlas_image_packs_bitmaps_by_slot() {
    let red = GlyphBitmap::solid_rgba8(
        GlyphEntry {
            slot: 0,
            generation: 0,
        },
        2,
        2,
        [255, 0, 0, 255],
    );
    let green = GlyphBitmap::solid_rgba8(
        GlyphEntry {
            slot: 1,
            generation: 0,
        },
        2,
        2,
        [0, 255, 0, 255],
    );

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
        width: 2,
        height: 2,
        rgba: vec![255; 4],
    };

    let error = GlyphAtlasImage::pack_rgba8(2, 2, 1, &[bad]).unwrap_err();

    assert!(error.contains("expected 16 rgba bytes"));
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

    assert!(error.contains("image dimensions are too large"));
}

#[test]
fn glyph_atlas_image_rejects_overflowing_dimensions_before_allocation() {
    let width_error = GlyphAtlasImage::pack_rgba8(u32::MAX, 1, 2, &[]).unwrap_err();
    assert!(width_error.contains("width is too large"));

    let tall_glyph = GlyphBitmap {
        entry: GlyphEntry {
            slot: 1,
            generation: 0,
        },
        width: 1,
        height: u32::MAX,
        rgba: Vec::new(),
    };
    let height_error = GlyphAtlasImage::pack_rgba8(1, u32::MAX, 1, &[tall_glyph]).unwrap_err();
    assert!(height_error.contains("height is too large"));

    let huge_slot = GlyphBitmap {
        entry: GlyphEntry {
            slot: u32::MAX - 1,
            generation: 0,
        },
        width: u32::MAX,
        height: 1,
        rgba: Vec::new(),
    };
    let image_error = GlyphAtlasImage::pack_rgba8(u32::MAX, 1, 1, &[huge_slot]).unwrap_err();
    assert!(image_error.contains("image dimensions are too large"));
}

#[test]
fn glyph_bitmap_padding_rejects_oversized_target_before_allocation() {
    let glyph = GlyphBitmap::solid_rgba8(
        GlyphEntry {
            slot: 0,
            generation: 0,
        },
        1,
        1,
        [255, 255, 255, 255],
    );

    let error = glyph.padded_to(u32::MAX, u32::MAX).unwrap_err();

    assert!(error.contains("image dimensions are too large"));
}
