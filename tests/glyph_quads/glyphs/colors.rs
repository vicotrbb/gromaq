use gromaq::renderer::{
    GlyphAtlas, GlyphAtlasConfig, GlyphQuadConfig, GlyphQuadPlanner, RenderPlanner,
};
use gromaq::{DEFAULT_ANSI_COLORS_RGB8, Terminal, TerminalConfig};

use crate::support::rgba;

#[test]
fn glyph_quad_planner_maps_terminal_style_to_foreground_rgba() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal
        .write_str(
            "\x1b[31mA\
             \x1b[38:2:17:34:51mB\
             \x1b[48:2:1:2:3;7mC\
             \x1b[27;2;38:2:100:120:140mD\
             \x1b[8mE",
        )
        .unwrap();
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
        atlas_columns: 5,
        atlas_width_px: 50,
        atlas_height_px: 20,
    };

    let batch = GlyphQuadPlanner::new(quad_config).plan(&plan).unwrap();

    assert_eq!(batch.quads.len(), 5);
    assert_eq!(
        batch.quads[0].vertices[0].foreground_rgba,
        rgba(255, 107, 122, 1.0)
    );
    assert_eq!(
        batch.quads[1].vertices[0].foreground_rgba,
        rgba(17, 34, 51, 1.0)
    );
    assert_eq!(
        batch.quads[2].vertices[0].foreground_rgba,
        rgba(1, 2, 3, 1.0)
    );
    assert_eq!(
        batch.quads[3].vertices[0].foreground_rgba,
        rgba(100, 120, 140, 0.42)
    );
    assert_eq!(
        batch.quads[4].vertices[0].foreground_rgba,
        [0.0, 0.0, 0.0, 0.0]
    );
}

#[test]
fn glyph_quad_planner_uses_configured_default_foreground_rgba() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());
    terminal.write_str("A").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut render_planner = RenderPlanner::with_default_foreground(14, [232, 226, 214]);
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
        atlas_columns: 1,
        atlas_width_px: 10,
        atlas_height_px: 20,
    };

    let batch = GlyphQuadPlanner::new(quad_config).plan(&plan).unwrap();

    assert_eq!(batch.quads.len(), 1);
    assert_eq!(
        batch.quads[0].vertices[0].foreground_rgba,
        rgba(232, 226, 214, 1.0)
    );
}
