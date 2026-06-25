use gromaq::renderer::{GpuRenderer, RendererConfig, WgpuRenderer};
use gromaq::{Terminal, TerminalConfig};

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
