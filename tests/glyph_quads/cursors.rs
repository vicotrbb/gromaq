use gromaq::renderer::{CursorQuadConfig, CursorQuadPlanner, RenderPlan};
use gromaq::{CursorShape, CursorSnapshot, DEFAULT_ANSI_COLORS_RGB8, DEFAULT_DIM_OPACITY};

use crate::support::rgba;

#[test]
fn cursor_quad_planner_builds_cursor_shapes() {
    let planner = CursorQuadPlanner::new(CursorQuadConfig {
        cell_width_px: 8,
        cell_height_px: 16,
        color_rgba8: [229, 229, 229, 255],
    });
    let mut plan = RenderPlan {
        viewport_cols: 8,
        viewport_rows: 3,
        cursor: CursorSnapshot {
            row: 1,
            col: 2,
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

    let block = planner.plan(&plan).unwrap();
    assert_eq!(block.quads.len(), 1);
    assert_eq!(block.indices, vec![0, 1, 2, 0, 2, 3]);
    assert_eq!(block.quads[0].vertices[0].position, [16.0, 16.0]);
    assert_eq!(block.quads[0].vertices[2].position, [24.0, 32.0]);
    assert_eq!(
        block.quads[0].vertices[0].color_rgba,
        rgba(229, 229, 229, 1.0)
    );

    plan.cursor.shape = CursorShape::Underline;
    let underline = planner.plan(&plan).unwrap();
    assert_eq!(underline.quads[0].vertices[0].position, [16.0, 30.0]);
    assert_eq!(underline.quads[0].vertices[2].position, [24.0, 32.0]);

    plan.cursor.shape = CursorShape::Bar;
    let bar = planner.plan(&plan).unwrap();
    assert_eq!(bar.quads[0].vertices[0].position, [16.0, 16.0]);
    assert_eq!(bar.quads[0].vertices[2].position, [17.0, 32.0]);
}

#[test]
fn cursor_quad_planner_skips_invisible_or_out_of_bounds_cursor() {
    let planner = CursorQuadPlanner::new(CursorQuadConfig {
        cell_width_px: 8,
        cell_height_px: 16,
        color_rgba8: [229, 229, 229, 255],
    });
    let mut plan = RenderPlan {
        viewport_cols: 8,
        viewport_rows: 3,
        cursor: CursorSnapshot {
            row: 1,
            col: 2,
            visible: false,
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

    assert!(planner.plan(&plan).unwrap().quads.is_empty());

    plan.cursor.visible = true;
    plan.cursor.col = 8;

    assert!(planner.plan(&plan).unwrap().quads.is_empty());
}
