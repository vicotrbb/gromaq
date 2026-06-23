use std::path::PathBuf;

use gromaq::font::RasterizedGlyphCache;
use gromaq::renderer::{GlyphAtlas, GlyphAtlasConfig, GlyphAtlasImage, RenderPlanner};
use gromaq::{Terminal, TerminalConfig};

fn system_mono_font() -> PathBuf {
    [
        "/System/Library/Fonts/SFNSMono.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|path| path.exists())
    .expect("expected a local macOS monospace font for renderer glyph rasterization proof")
}

#[test]
fn rasterized_glyph_cache_populates_distinct_plan_glyphs_once() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    terminal.write_str("ABA").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut planner = RenderPlanner::new(18);
    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();
    let font_bytes = std::fs::read(system_mono_font()).unwrap();
    let mut cache = RasterizedGlyphCache::from_bytes(font_bytes).unwrap();

    let batch = cache.rasterize_plan(&plan).unwrap();

    assert_eq!(plan.glyphs.len(), 3);
    assert_eq!(batch.rasterized, 2);
    assert_eq!(batch.reused, 1);
    assert_eq!(batch.bitmaps.len(), 2);
    assert_eq!(cache.len(), 2);
    assert!(batch.bitmaps.iter().all(|glyph| glyph.width > 0));
    assert!(batch.bitmaps.iter().all(|glyph| glyph.height > 0));
    assert!(
        batch
            .bitmaps
            .iter()
            .all(|glyph| glyph.rgba.chunks_exact(4).any(|pixel| pixel[3] > 0))
    );

    let slot_width = batch.bitmaps.iter().map(|glyph| glyph.width).max().unwrap();
    let slot_height = batch
        .bitmaps
        .iter()
        .map(|glyph| glyph.height)
        .max()
        .unwrap();
    let normalized: Vec<_> = batch
        .bitmaps
        .into_iter()
        .map(|glyph| glyph.padded_to(slot_width, slot_height).unwrap())
        .collect();
    let image = GlyphAtlasImage::pack_rgba8(slot_width, slot_height, 2, &normalized).unwrap();
    assert_eq!(image.occupied_slots, 2);
    assert!(image.rgba.chunks_exact(4).any(|pixel| pixel[3] > 0));
}

#[test]
fn rasterized_glyph_cache_returns_cached_bitmaps_for_reused_plan() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    terminal.write_str("ABA").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut planner = RenderPlanner::new(18);
    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();
    let font_bytes = std::fs::read(system_mono_font()).unwrap();
    let mut cache = RasterizedGlyphCache::from_bytes(font_bytes).unwrap();

    let first = cache.rasterize_plan(&plan).unwrap();
    let second = cache.rasterize_plan(&plan).unwrap();

    assert_eq!(first.rasterized, 2);
    assert_eq!(first.reused, 1);
    assert_eq!(second.rasterized, 0);
    assert_eq!(second.reused, 3);
    assert_eq!(second.bitmaps.len(), 2);
    assert!(second.bitmaps.iter().all(|glyph| glyph.width > 0));
}
