use gromaq::renderer::{
    BackgroundQuadConfig, BackgroundQuadError, BackgroundQuadPlanner, CursorQuadConfig,
    CursorQuadPlanner, GlyphAtlas, GlyphAtlasConfig, GlyphEntry, GlyphQuadConfig, GlyphQuadError,
    GlyphQuadPlanner, PlannedGlyph, PlannedTextDecoration, RenderPlan, RenderPlanner,
    TextDecorationKind, TextDecorationQuadConfig, TextDecorationQuadPlanner,
};
use gromaq::{
    CursorShape, CursorSnapshot, DEFAULT_ANSI_COLORS_RGB8, DEFAULT_DIM_OPACITY, Style, Terminal,
    TerminalConfig,
};

fn rgba(red: u8, green: u8, blue: u8, alpha: f32) -> [f32; 4] {
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

fn triangle_indices_for_quads(quad_count: usize) -> Vec<u32> {
    let mut indices = Vec::with_capacity(quad_count * 6);
    for quad_index in 0..quad_count {
        let base = u32::try_from(quad_index * 4).unwrap();
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    indices
}

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
    assert_eq!(second.vertices[0].color_rgba, rgba(130, 170, 255, 1.0));
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

#[test]
fn glyph_quad_planner_rejects_invalid_atlas_dimensions() {
    let invalid = GlyphQuadConfig {
        cell_width_px: 8,
        cell_height_px: 16,
        atlas_slot_width_px: 10,
        atlas_slot_height_px: 20,
        atlas_columns: 0,
        atlas_width_px: 20,
        atlas_height_px: 20,
    };

    assert!(
        GlyphQuadPlanner::new(invalid)
            .plan(&RenderPlan {
                viewport_cols: 0,
                viewport_rows: 0,
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
            })
            .is_err()
    );
}

#[test]
fn glyph_quad_planner_rejects_slots_outside_the_atlas_image() {
    let config = GlyphQuadConfig {
        cell_width_px: 8,
        cell_height_px: 16,
        atlas_slot_width_px: 10,
        atlas_slot_height_px: 20,
        atlas_columns: 1,
        atlas_width_px: 10,
        atlas_height_px: 20,
    };
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
        glyphs: vec![PlannedGlyph {
            row: 0,
            col: 0,
            text: "B".to_owned(),
            ch: 'B',
            style: Style::default(),
            font_size_px: 14,
            is_wide: false,
            atlas_entry: GlyphEntry {
                slot: 1,
                generation: 0,
            },
        }],
    };

    assert_eq!(
        GlyphQuadPlanner::new(config).plan(&plan).unwrap_err(),
        GlyphQuadError::SlotOutsideAtlas { slot: 1 }
    );
}

#[test]
fn glyph_quad_planner_rejects_overflowing_atlas_coordinates() {
    let config = GlyphQuadConfig {
        cell_width_px: 8,
        cell_height_px: 16,
        atlas_slot_width_px: u32::MAX,
        atlas_slot_height_px: 20,
        atlas_columns: 1,
        atlas_width_px: u32::MAX,
        atlas_height_px: 20,
    };
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
        glyphs: vec![PlannedGlyph {
            row: 0,
            col: 0,
            text: "B".to_owned(),
            ch: 'B',
            style: Style::default(),
            font_size_px: 14,
            is_wide: false,
            atlas_entry: GlyphEntry {
                slot: 1,
                generation: 0,
            },
        }],
    };

    assert_eq!(
        GlyphQuadPlanner::new(config).plan(&plan).unwrap_err(),
        GlyphQuadError::SlotOutsideAtlas { slot: 1 }
    );
}
