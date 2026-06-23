use gromaq::renderer::{
    BackgroundQuadConfig, BackgroundQuadError, BackgroundQuadPlanner, CursorQuadConfig,
    CursorQuadPlanner, GlyphAtlas, GlyphAtlasConfig, GlyphEntry, GlyphQuadConfig, GlyphQuadError,
    GlyphQuadPlanner, PlannedGlyph, RenderPlan, RenderPlanner,
};
use gromaq::{CursorShape, CursorSnapshot, Style, Terminal, TerminalConfig};

fn rgba(red: u8, green: u8, blue: u8, alpha: f32) -> [f32; 4] {
    [
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        alpha,
    ]
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
        atlas_columns: 5,
        atlas_width_px: 50,
        atlas_height_px: 20,
    };

    let batch = GlyphQuadPlanner::new(quad_config).plan(&plan).unwrap();

    assert_eq!(batch.quads.len(), 5);
    assert_eq!(
        batch.quads[0].vertices[0].foreground_rgba,
        rgba(205, 49, 49, 1.0)
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
        rgba(100, 120, 140, 0.66)
    );
    assert_eq!(
        batch.quads[4].vertices[0].foreground_rgba,
        [0.0, 0.0, 0.0, 0.0]
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
    assert_eq!(second.vertices[0].color_rgba, rgba(36, 114, 200, 1.0));
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
        clear_regions: Vec::new(),
        backgrounds: Vec::new(),
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
        clear_regions: Vec::new(),
        backgrounds: Vec::new(),
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
        clear_regions: Vec::new(),
        backgrounds: Vec::new(),
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
                clear_regions: Vec::new(),
                backgrounds: Vec::new(),
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
        clear_regions: Vec::new(),
        backgrounds: Vec::new(),
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
        clear_regions: Vec::new(),
        backgrounds: Vec::new(),
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
