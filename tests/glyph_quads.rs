use gromaq::renderer::{
    GlyphAtlas, GlyphAtlasConfig, GlyphEntry, GlyphQuadConfig, GlyphQuadError, GlyphQuadPlanner,
    PlannedGlyph, RenderPlan, RenderPlanner,
};
use gromaq::{CursorShape, CursorSnapshot, Style, Terminal, TerminalConfig};

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
