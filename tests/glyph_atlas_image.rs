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
