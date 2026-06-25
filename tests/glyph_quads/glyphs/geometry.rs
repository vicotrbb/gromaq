use gromaq::renderer::{
    GlyphAtlas, GlyphAtlasConfig, GlyphQuadConfig, GlyphQuadPlanner, RenderPlanner,
};
use gromaq::{DEFAULT_ANSI_COLORS_RGB8, Terminal, TerminalConfig};

#[test]
fn glyph_quad_planner_builds_positioned_quads_with_atlas_uvs() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("A界").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut render_planner = RenderPlanner::new(14);
    let plan = render_planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();
    let quad_config = GlyphQuadConfig {
        cell_width_px: 8,
        cell_height_px: 16,
        atlas_slot_width_px: 10,
        atlas_slot_height_px: 20,
        atlas_columns: 2,
        atlas_width_px: 20,
        atlas_height_px: 20,
    };

    let batch = GlyphQuadPlanner::new(quad_config).plan(&plan).unwrap();

    assert_eq!(batch.quads.len(), 2);
    assert_eq!(batch.indices, vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);

    let first = &batch.quads[0];
    assert_eq!(first.ch, 'A');
    assert_eq!(first.vertices[0].position, [0.0, 0.0]);
    assert_eq!(first.vertices[1].position, [8.0, 0.0]);
    assert_eq!(first.vertices[2].position, [8.0, 16.0]);
    assert_eq!(first.vertices[3].position, [0.0, 16.0]);
    assert_eq!(first.vertices[0].uv, [0.0, 0.0]);
    assert_eq!(first.vertices[2].uv, [0.5, 1.0]);

    let wide = &batch.quads[1];
    assert_eq!(wide.ch, '界');
    assert_eq!(wide.vertices[0].position, [8.0, 0.0]);
    assert_eq!(wide.vertices[1].position, [24.0, 0.0]);
    assert_eq!(wide.vertices[2].position, [24.0, 16.0]);
    assert_eq!(wide.vertices[3].position, [8.0, 16.0]);
    assert_eq!(wide.vertices[0].uv, [0.5, 0.0]);
    assert_eq!(wide.vertices[2].uv, [1.0, 1.0]);
}

#[test]
fn glyph_quad_planner_preserves_multi_codepoint_cell_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("👨\u{200d}👩").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut render_planner = RenderPlanner::with_visual_theme(
        14,
        [229, 229, 229],
        DEFAULT_ANSI_COLORS_RGB8,
        [43, 65, 98, 255],
        0.42,
    );
    let plan = render_planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();
    let quad_config = GlyphQuadConfig {
        cell_width_px: 8,
        cell_height_px: 16,
        atlas_slot_width_px: 10,
        atlas_slot_height_px: 20,
        atlas_columns: 2,
        atlas_width_px: 20,
        atlas_height_px: 20,
    };

    let batch = GlyphQuadPlanner::new(quad_config).plan(&plan).unwrap();

    assert_eq!(batch.quads.len(), 1);
    assert_eq!(batch.quads[0].text, "👨\u{200d}👩");
    assert_eq!(batch.quads[0].ch, '👨');
    assert_eq!(batch.quads[0].vertices[1].position, [16.0, 0.0]);
}
