use gromaq::font::RasterizedGlyphCache;
use gromaq::renderer::{
    GpuRenderer, PreparedSurfaceGlyphFrame, PreparedSurfaceGlyphFrameConfig, RendererConfig,
    WgpuRenderer,
};
use gromaq::{Terminal, TerminalConfig};

use crate::support::{linear_rgba, system_mono_font};

#[test]
fn prepared_surface_glyph_frame_builds_from_render_plan_and_rasterized_glyphs() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    terminal.write_str("\x1b[48:2:1:2:3;4mABA").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut renderer = WgpuRenderer::new(RendererConfig::default()).unwrap();
    renderer
        .render_frame(&terminal.dump_grid(), terminal.dump_cursor(), &dirty)
        .unwrap();
    let plan = renderer.last_plan().unwrap();
    let font_bytes = std::fs::read(system_mono_font()).unwrap();
    let mut glyph_cache = RasterizedGlyphCache::from_bytes(font_bytes).unwrap();
    let glyphs = glyph_cache.rasterize_plan(plan).unwrap();

    let prepared = PreparedSurfaceGlyphFrame::from_render_plan(
        plan,
        &glyphs.bitmaps,
        PreparedSurfaceGlyphFrameConfig {
            cell_width_px: renderer.config().cell_width_px,
            line_height_px: renderer.config().line_height_px,
            clear_color: [0.0, 0.0, 0.0, 1.0],
            cursor_color_rgba8: [244, 192, 106, 255],
            surface_padding_px: 12,
            cell_spacing_px: 0,
        },
    )
    .unwrap();
    let frame = prepared.as_surface_glyph_frame();

    assert_eq!(frame.batch.quads.len(), plan.glyphs.len());
    assert_eq!(frame.batch.indices.len(), plan.glyphs.len() * 6);
    assert_eq!(frame.background_batch.quads.len(), 1);
    assert_eq!(frame.background_batch.indices.len(), 6);
    assert_eq!(frame.decoration_batch.quads.len(), 1);
    assert_eq!(frame.decoration_batch.indices.len(), 6);
    assert_eq!(frame.cursor_batch.quads.len(), 1);
    assert_eq!(frame.cursor_batch.indices.len(), 6);
    assert_eq!(frame.batch.quads[0].vertices[0].position, [12.0, 12.0]);
    assert_eq!(
        frame.background_batch.quads[0].vertices[0].position,
        [12.0, 12.0]
    );
    let planned_cell_width = (frame.width - 24) as f32 / f32::from(plan.viewport_cols);
    assert_eq!(
        frame.cursor_batch.quads[0].vertices[0].position,
        [12.0 + f32::from(plan.cursor.col) * planned_cell_width, 12.0]
    );
    assert_eq!(
        frame.cursor_batch.quads[0].vertices[0].color_rgba,
        linear_rgba(244, 192, 106, 1.0)
    );
    assert_eq!(frame.atlas.occupied_slots, 2);
    assert!(frame.width >= 24);
    assert!(frame.height >= 24);
    assert!(frame.atlas.rgba.chunks_exact(4).any(|pixel| pixel[3] > 0));
}
