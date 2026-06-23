use std::path::PathBuf;

use gromaq::font::{FontRasterError, FontRasterizer};
use gromaq::renderer::{GlyphAtlasImage, GlyphEntry};

fn system_mono_font() -> PathBuf {
    [
        "/System/Library/Fonts/SFNSMono.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
        "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|path| path.exists())
    .expect("expected a local system monospace font for rasterization proof")
}

#[test]
fn font_rasterizer_renders_real_font_glyph_to_atlas_bitmap() {
    let font_bytes = std::fs::read(system_mono_font()).unwrap();
    let mut rasterizer = FontRasterizer::from_bytes(font_bytes).unwrap();
    let entry = GlyphEntry {
        slot: 0,
        generation: 0,
    };

    let glyph = rasterizer.rasterize('A', 18.0, entry).unwrap();

    assert_eq!(glyph.entry, entry);
    assert!(glyph.width > 0);
    assert!(glyph.height > 0);
    assert_eq!(
        glyph.rgba.len(),
        glyph.width as usize * glyph.height as usize * 4
    );
    assert!(glyph.rgba.chunks_exact(4).any(|pixel| pixel[3] > 0));

    let atlas = GlyphAtlasImage::pack_rgba8(glyph.width, glyph.height, 1, &[glyph]).unwrap();
    assert_eq!(atlas.occupied_slots, 1);
    assert!(atlas.rgba.chunks_exact(4).any(|pixel| pixel[3] > 0));
}

#[test]
fn font_rasterizer_reports_missing_visible_glyph() {
    let font_bytes = std::fs::read(system_mono_font()).unwrap();
    let mut rasterizer = FontRasterizer::from_bytes(font_bytes).unwrap();
    let entry = GlyphEntry {
        slot: 0,
        generation: 0,
    };

    let error = rasterizer.rasterize('\u{10ffff}', 18.0, entry).unwrap_err();

    assert_eq!(error, FontRasterError::MissingGlyph('\u{10ffff}'));
}
