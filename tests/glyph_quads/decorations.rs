use gromaq::renderer::{
    BackgroundQuadError, GlyphAtlas, GlyphAtlasConfig, PlannedTextDecoration, RenderPlan,
    RenderPlanner, TextDecorationKind, TextDecorationQuadConfig, TextDecorationQuadPlanner,
};
use gromaq::{
    CursorShape, CursorSnapshot, DEFAULT_ANSI_COLORS_RGB8, DEFAULT_DIM_OPACITY, Terminal,
    TerminalConfig,
};

use crate::support::{rgba, triangle_indices_for_quads};

#[test]
fn render_planner_extracts_text_decorations() {
    let mut terminal = Terminal::new(TerminalConfig::new(18, 2).unwrap());
    terminal
        .write_str(
            "\x1b[4;58:2:17:34:51mAB\
             \x1b[0;21mCD\
             \x1b[0;53mE\
             \x1b[0;9mF\
             \x1b[0;8;4mG\
             \x1b[0m\x1b[4:3mHI\
             \x1b[0m\x1b[4:4mJK\
             \x1b[0m\x1b[4:5mLM\
             \x1b[0m\x1b[8m\x1b[4:5mN",
        )
        .unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(12).unwrap());
    let mut render_planner = RenderPlanner::new(14);

    let plan = render_planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();

    assert_eq!(
        plan.decorations,
        vec![
            PlannedTextDecoration {
                row: 0,
                col: 0,
                cols: 2,
                kind: TextDecorationKind::Underline,
                color_rgba8: [17, 34, 51, 255],
            },
            PlannedTextDecoration {
                row: 0,
                col: 2,
                cols: 2,
                kind: TextDecorationKind::DoubleUnderlineTop,
                color_rgba8: [229, 229, 229, 255],
            },
            PlannedTextDecoration {
                row: 0,
                col: 2,
                cols: 2,
                kind: TextDecorationKind::DoubleUnderlineBottom,
                color_rgba8: [229, 229, 229, 255],
            },
            PlannedTextDecoration {
                row: 0,
                col: 4,
                cols: 1,
                kind: TextDecorationKind::Overline,
                color_rgba8: [229, 229, 229, 255],
            },
            PlannedTextDecoration {
                row: 0,
                col: 5,
                cols: 1,
                kind: TextDecorationKind::Strikethrough,
                color_rgba8: [229, 229, 229, 255],
            },
            PlannedTextDecoration {
                row: 0,
                col: 7,
                cols: 2,
                kind: TextDecorationKind::CurlyUnderline,
                color_rgba8: [229, 229, 229, 255],
            },
            PlannedTextDecoration {
                row: 0,
                col: 9,
                cols: 2,
                kind: TextDecorationKind::DottedUnderline,
                color_rgba8: [229, 229, 229, 255],
            },
            PlannedTextDecoration {
                row: 0,
                col: 11,
                cols: 2,
                kind: TextDecorationKind::DashedUnderline,
                color_rgba8: [229, 229, 229, 255],
            },
        ]
    );
}

#[test]
fn text_decoration_quad_planner_builds_line_geometry() {
    let plan = RenderPlan {
        viewport_cols: 4,
        viewport_rows: 2,
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
        decorations: vec![
            PlannedTextDecoration {
                row: 0,
                col: 0,
                cols: 2,
                kind: TextDecorationKind::Underline,
                color_rgba8: [255, 0, 0, 255],
            },
            PlannedTextDecoration {
                row: 0,
                col: 2,
                cols: 1,
                kind: TextDecorationKind::DoubleUnderlineTop,
                color_rgba8: [0, 255, 0, 255],
            },
            PlannedTextDecoration {
                row: 0,
                col: 3,
                cols: 1,
                kind: TextDecorationKind::DoubleUnderlineBottom,
                color_rgba8: [0, 0, 255, 255],
            },
            PlannedTextDecoration {
                row: 1,
                col: 0,
                cols: 1,
                kind: TextDecorationKind::Overline,
                color_rgba8: [255, 255, 0, 255],
            },
            PlannedTextDecoration {
                row: 1,
                col: 1,
                cols: 1,
                kind: TextDecorationKind::Strikethrough,
                color_rgba8: [255, 0, 255, 255],
            },
        ],
        glyphs: Vec::new(),
    };

    let batch = TextDecorationQuadPlanner::new(TextDecorationQuadConfig {
        cell_width_px: 8,
        cell_height_px: 20,
    })
    .plan(&plan)
    .unwrap();

    assert_eq!(batch.quads.len(), 5);
    assert_eq!(
        batch.indices,
        vec![
            0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16,
            17, 18, 16, 18, 19,
        ]
    );
    assert_eq!(batch.quads[0].vertices[0].position, [0.0, 18.0]);
    assert_eq!(batch.quads[0].vertices[2].position, [16.0, 20.0]);
    assert_eq!(batch.quads[0].vertices[0].color_rgba, rgba(255, 0, 0, 1.0));
    assert_eq!(batch.quads[1].vertices[0].position, [16.0, 14.0]);
    assert_eq!(batch.quads[1].vertices[2].position, [24.0, 16.0]);
    assert_eq!(batch.quads[2].vertices[0].position, [24.0, 18.0]);
    assert_eq!(batch.quads[2].vertices[2].position, [32.0, 20.0]);
    assert_eq!(batch.quads[3].vertices[0].position, [0.0, 20.0]);
    assert_eq!(batch.quads[3].vertices[2].position, [8.0, 22.0]);
    assert_eq!(batch.quads[4].vertices[0].position, [8.0, 29.0]);
    assert_eq!(batch.quads[4].vertices[2].position, [16.0, 31.0]);
}

#[test]
fn text_decoration_quad_planner_builds_styled_underline_geometry() {
    let plan = RenderPlan {
        viewport_cols: 6,
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
        decorations: vec![
            PlannedTextDecoration {
                row: 0,
                col: 0,
                cols: 2,
                kind: TextDecorationKind::DottedUnderline,
                color_rgba8: [255, 0, 0, 255],
            },
            PlannedTextDecoration {
                row: 0,
                col: 2,
                cols: 2,
                kind: TextDecorationKind::DashedUnderline,
                color_rgba8: [0, 255, 0, 255],
            },
            PlannedTextDecoration {
                row: 0,
                col: 4,
                cols: 2,
                kind: TextDecorationKind::CurlyUnderline,
                color_rgba8: [0, 0, 255, 255],
            },
        ],
        glyphs: Vec::new(),
    };

    let batch = TextDecorationQuadPlanner::new(TextDecorationQuadConfig {
        cell_width_px: 8,
        cell_height_px: 20,
    })
    .plan(&plan)
    .unwrap();

    assert_eq!(batch.quads.len(), 10);
    assert_eq!(batch.indices, triangle_indices_for_quads(10));
    assert_eq!(batch.quads[0].vertices[0].position, [0.0, 18.0]);
    assert_eq!(batch.quads[0].vertices[2].position, [2.0, 20.0]);
    assert_eq!(batch.quads[1].vertices[0].position, [4.0, 18.0]);
    assert_eq!(batch.quads[3].vertices[2].position, [14.0, 20.0]);
    assert_eq!(batch.quads[4].vertices[0].position, [16.0, 18.0]);
    assert_eq!(batch.quads[4].vertices[2].position, [22.0, 20.0]);
    assert_eq!(batch.quads[5].vertices[0].position, [26.0, 18.0]);
    assert_eq!(batch.quads[5].vertices[2].position, [32.0, 20.0]);
    assert!(batch.quads[6].vertices[0].position[1] > batch.quads[6].vertices[1].position[1]);
    assert!(batch.quads[7].vertices[0].position[1] < batch.quads[7].vertices[1].position[1]);
    assert_eq!(batch.quads[0].vertices[0].color_rgba, rgba(255, 0, 0, 1.0));
    assert_eq!(batch.quads[4].vertices[0].color_rgba, rgba(0, 255, 0, 1.0));
    assert_eq!(batch.quads[6].vertices[0].color_rgba, rgba(0, 0, 255, 1.0));
}

#[test]
fn text_decoration_quad_planner_rejects_invalid_dimensions() {
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
        TextDecorationQuadPlanner::new(TextDecorationQuadConfig {
            cell_width_px: 8,
            cell_height_px: 0,
        })
        .plan(&plan)
        .unwrap_err(),
        BackgroundQuadError::ZeroDimension
    );
}
