use std::path::PathBuf;

use gromaq::font::RasterizedGlyphCache;
use gromaq::renderer::{GpuRenderer, PreparedSurfaceGlyphFrame, RendererConfig, WgpuRenderer};
use gromaq::{GromaqConfig, Terminal, TerminalConfig};

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
    .expect("expected a local system monospace font for renderer glyph frame proof")
}

#[test]
fn wgpu_renderer_records_last_planned_frame() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    terminal.write_str("abcd").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut renderer = WgpuRenderer::new(RendererConfig::default()).unwrap();

    renderer
        .render_frame(&terminal.dump_grid(), terminal.dump_cursor(), &dirty)
        .unwrap();

    let plan = renderer
        .last_plan()
        .expect("renderer should keep last plan");
    let planned: Vec<(u16, u16, char)> = plan
        .glyphs
        .iter()
        .map(|glyph| (glyph.row, glyph.col, glyph.ch))
        .collect();
    assert_eq!(
        planned,
        vec![(0, 0, 'a'), (0, 1, 'b'), (0, 2, 'c'), (0, 3, 'd')]
    );
    assert_eq!(renderer.glyph_atlas_metrics().entries, 4);
}

fn linear_clear_color(red: u8, green: u8, blue: u8) -> [f64; 4] {
    [
        f64::from(srgb8_to_linear_f32(red)),
        f64::from(srgb8_to_linear_f32(green)),
        f64::from(srgb8_to_linear_f32(blue)),
        1.0,
    ]
}

fn linear_rgba(red: u8, green: u8, blue: u8, alpha: f32) -> [f32; 4] {
    [
        srgb8_to_linear_f32(red),
        srgb8_to_linear_f32(green),
        srgb8_to_linear_f32(blue),
        alpha,
    ]
}

fn srgb8_to_linear_f32(value: u8) -> f32 {
    let srgb = f32::from(value) / 255.0;
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

#[test]
fn wgpu_renderer_uses_configured_font_size_for_render_plan() {
    let config = RendererConfig {
        font_size_px: 18,
        ..RendererConfig::default()
    };
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());
    terminal.write_str("A").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut renderer = WgpuRenderer::new(config).unwrap();

    renderer
        .render_frame(&terminal.dump_grid(), terminal.dump_cursor(), &dirty)
        .unwrap();

    let plan = renderer.last_plan().unwrap();
    assert_eq!(plan.glyphs.len(), 1);
    assert_eq!(plan.glyphs[0].font_size_px, 18);
}

#[test]
fn renderer_config_maps_validated_gromaq_settings() {
    let mut config = GromaqConfig::default();
    config.performance.target_fps = 120;
    config.performance.dirty_region_rendering = false;
    config.font.size_px = 16.5;
    config.font.cell_width_px = Some(9.5);
    config.font.line_height_px = 21.0;
    config.theme.background = "#1f2028".to_owned();
    config.theme.foreground = "#e8e2d6".to_owned();
    config.theme.cursor = "#f4c06a".to_owned();
    config.theme.selection = "#26364f".to_owned();
    config.theme.ansi[1] = "#010203".to_owned();
    config.theme.surface_padding_px = 18;

    let renderer_config = RendererConfig::from_gromaq_config(&config).unwrap();

    assert_eq!(renderer_config.target_fps, 120);
    assert!(!renderer_config.dirty_regions);
    assert_eq!(renderer_config.font_size_px, 17);
    assert_eq!(renderer_config.cell_width_px, 10);
    assert_eq!(renderer_config.line_height_px, 21);
    assert_eq!(renderer_config.clear_color, linear_clear_color(31, 32, 40));
    assert_eq!(renderer_config.default_foreground_rgb8, [232, 226, 214]);
    assert_eq!(renderer_config.ansi_colors_rgb8[1], [1, 2, 3]);
    assert_eq!(renderer_config.cursor_color_rgba8, [244, 192, 106, 255]);
    assert_eq!(
        renderer_config.selection_background_rgba8,
        [38, 54, 79, 255]
    );
    assert_eq!(renderer_config.surface_padding_px, 18);
}

#[test]
fn renderer_default_cell_width_is_compact_for_monospace_text() {
    let config = RendererConfig::default();

    assert_eq!(config.font_size_px, 16);
    assert_eq!(config.cell_width_px, 10);
    assert!(config.cell_width_px < config.font_size_px);
}

#[test]
fn renderer_default_theme_matches_default_gromaq_config() {
    let default_renderer = RendererConfig::default();
    let mapped_renderer = RendererConfig::from_gromaq_config(&GromaqConfig::default()).unwrap();

    assert_eq!(default_renderer.clear_color, mapped_renderer.clear_color);
    assert_eq!(
        default_renderer.default_foreground_rgb8,
        mapped_renderer.default_foreground_rgb8
    );
    assert_eq!(
        default_renderer.cursor_color_rgba8,
        mapped_renderer.cursor_color_rgba8
    );
    assert_eq!(
        default_renderer.selection_background_rgba8,
        mapped_renderer.selection_background_rgba8
    );
    assert_eq!(
        default_renderer.ansi_colors_rgb8,
        mapped_renderer.ansi_colors_rgb8
    );
    assert_eq!(
        default_renderer.surface_padding_px,
        mapped_renderer.surface_padding_px
    );
    assert_eq!(
        default_renderer.cell_width_px,
        mapped_renderer.cell_width_px
    );
}

#[test]
fn wgpu_renderer_reconfigure_updates_future_frame_planning() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());
    terminal.write_str("A").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut renderer = WgpuRenderer::new(RendererConfig::default()).unwrap();
    renderer
        .render_frame(&terminal.dump_grid(), terminal.dump_cursor(), &dirty)
        .unwrap();
    assert!(renderer.last_plan().is_some());

    renderer.reconfigure(RendererConfig {
        font_size_px: 20,
        dirty_regions: false,
        ..RendererConfig::default()
    });
    assert!(renderer.last_plan().is_none());

    terminal.write_str("\rB").unwrap();
    let dirty = terminal.take_dirty_regions();
    renderer
        .render_frame(&terminal.dump_grid(), terminal.dump_cursor(), &dirty)
        .unwrap();

    let plan = renderer.last_plan().unwrap();
    assert_eq!(plan.glyphs[0].font_size_px, 20);
    assert_eq!(plan.clear_regions.len(), 1);
    assert_eq!(plan.clear_regions[0].rows, 2);
    assert_eq!(plan.clear_regions[0].cols, 4);
}

#[test]
fn wgpu_renderer_can_plan_full_viewport_when_dirty_regions_are_disabled() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    terminal.write_str("abcd").unwrap();
    terminal.take_dirty_regions();
    terminal.write_str("\r\x1b[2CZ").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut renderer = WgpuRenderer::new(RendererConfig {
        dirty_regions: false,
        ..RendererConfig::default()
    })
    .unwrap();

    renderer
        .render_frame(&terminal.dump_grid(), terminal.dump_cursor(), &dirty)
        .unwrap();

    let plan = renderer
        .last_plan()
        .expect("renderer should keep last plan");
    let planned: Vec<(u16, u16, char)> = plan
        .glyphs
        .iter()
        .map(|glyph| (glyph.row, glyph.col, glyph.ch))
        .collect();
    assert_eq!(
        planned,
        vec![(0, 0, 'a'), (0, 1, 'b'), (0, 2, 'Z'), (0, 3, 'd')]
    );
    assert_eq!(plan.clear_regions.len(), 1);
    assert_eq!(plan.clear_regions[0].row, 0);
    assert_eq!(plan.clear_regions[0].col, 0);
    assert_eq!(plan.clear_regions[0].rows, 2);
    assert_eq!(plan.clear_regions[0].cols, 8);
}

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
        renderer.config().cell_width_px,
        renderer.config().line_height_px,
        [0.0, 0.0, 0.0, 1.0],
        [244, 192, 106, 255],
        12,
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
