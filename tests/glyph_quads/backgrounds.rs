use gromaq::renderer::{
    BackgroundQuadConfig, BackgroundQuadError, BackgroundQuadPlanner, GlyphAtlas, GlyphAtlasConfig,
    RenderPlan, RenderPlanner,
};
use gromaq::{
    CursorShape, CursorSnapshot, DEFAULT_ANSI_COLORS_RGB8, DEFAULT_DIM_OPACITY, Terminal,
    TerminalConfig,
};

use crate::support::rgba;

#[test]
fn background_quad_planner_builds_solid_cell_spans() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal
        .write_str("\x1b[48:2:1:2:3mAB \x1b[0mC\x1b[44mD")
        .unwrap();
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

    let batch = BackgroundQuadPlanner::new(BackgroundQuadConfig {
        cell_width_px: 8,
        cell_height_px: 16,
    })
    .plan(&plan)
    .unwrap();

    assert_eq!(batch.quads.len(), 2);
    assert_eq!(batch.indices, vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);

    let first = &batch.quads[0];
    assert_eq!(first.row, 0);
    assert_eq!(first.col, 0);
    assert_eq!(first.cols, 3);
    assert_eq!(first.vertices[0].position, [0.0, 0.0]);
    assert_eq!(first.vertices[1].position, [24.0, 0.0]);
    assert_eq!(first.vertices[2].position, [24.0, 16.0]);
    assert_eq!(first.vertices[0].color_rgba, rgba(1, 2, 3, 1.0));

    let second = &batch.quads[1];
    assert_eq!(second.row, 0);
    assert_eq!(second.col, 4);
    assert_eq!(second.cols, 1);
    assert_eq!(second.vertices[0].position, [32.0, 0.0]);
    assert_eq!(second.vertices[1].position, [40.0, 0.0]);
    assert_eq!(second.vertices[0].color_rgba, rgba(122, 162, 247, 1.0));
}

#[test]
fn background_quad_planner_rejects_invalid_dimensions() {
    let plan = RenderPlan {
        viewport_cols: 1,
        viewport_rows: 1,
        cursor: CursorSnapshot {
            row: 0,
            col: 0,
            visible: true,
            shape: CursorShape::Block,
            blinking: true,
        },
        default_foreground_rgb8: [229, 229, 229],
        ansi_colors_rgb8: DEFAULT_ANSI_COLORS_RGB8,
        dim_opacity: DEFAULT_DIM_OPACITY,
        clear_regions: Vec::new(),
        backgrounds: Vec::new(),
        decorations: Vec::new(),
        glyphs: Vec::new(),
    };

    assert_eq!(
        BackgroundQuadPlanner::new(BackgroundQuadConfig {
            cell_width_px: 0,
            cell_height_px: 16,
        })
        .plan(&plan)
        .unwrap_err(),
        BackgroundQuadError::ZeroDimension
    );
}
