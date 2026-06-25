use std::path::PathBuf;

use super::*;

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
fn default_rasterizer_emboldens_outline_glyph_alpha_coverage() {
    let font_bytes = std::fs::read(system_mono_font()).unwrap();
    let mut plain =
        FontRasterizer::from_bytes_with_outline_embolden(font_bytes.clone(), 0.0).unwrap();
    let mut emboldened = FontRasterizer::from_bytes(font_bytes).unwrap();
    let plain_glyph = plain
        .rasterize(
            'm',
            28.0,
            GlyphEntry {
                slot: 0,
                generation: 0,
            },
        )
        .unwrap();
    let emboldened_glyph = emboldened
        .rasterize(
            'm',
            28.0,
            GlyphEntry {
                slot: 0,
                generation: 0,
            },
        )
        .unwrap();

    assert!(alpha_coverage(&emboldened_glyph.rgba) > alpha_coverage(&plain_glyph.rgba));
}

fn alpha_coverage(rgba: &[u8]) -> u64 {
    rgba.chunks_exact(4).map(|pixel| u64::from(pixel[3])).sum()
}
