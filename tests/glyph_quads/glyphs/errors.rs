use gromaq::renderer::{
    GlyphEntry, GlyphQuadConfig, GlyphQuadError, GlyphQuadPlanner, PlannedGlyph, RenderPlan,
};
use gromaq::{CursorShape, CursorSnapshot, DEFAULT_ANSI_COLORS_RGB8, DEFAULT_DIM_OPACITY, Style};

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

    assert!(GlyphQuadPlanner::new(invalid).plan(&empty_plan()).is_err());
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

    assert_eq!(
        GlyphQuadPlanner::new(config)
            .plan(&single_glyph_plan_with_slot(1))
            .unwrap_err(),
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

    assert_eq!(
        GlyphQuadPlanner::new(config)
            .plan(&single_glyph_plan_with_slot(1))
            .unwrap_err(),
        GlyphQuadError::SlotOutsideAtlas { slot: 1 }
    );
}

fn empty_plan() -> RenderPlan {
    RenderPlan {
        viewport_cols: 0,
        viewport_rows: 0,
        cursor: default_cursor(),
        default_foreground_rgb8: [229, 229, 229],
        ansi_colors_rgb8: DEFAULT_ANSI_COLORS_RGB8,
        dim_opacity: DEFAULT_DIM_OPACITY,
        clear_regions: Vec::new(),
        backgrounds: Vec::new(),
        decorations: Vec::new(),
        glyphs: Vec::new(),
    }
}

fn single_glyph_plan_with_slot(slot: u32) -> RenderPlan {
    RenderPlan {
        viewport_cols: 1,
        viewport_rows: 1,
        cursor: default_cursor(),
        glyphs: vec![PlannedGlyph {
            row: 0,
            col: 0,
            text: "B".to_owned(),
            ch: 'B',
            style: Style::default(),
            font_size_px: 14,
            is_wide: false,
            atlas_entry: GlyphEntry {
                slot,
                generation: 0,
            },
        }],
        ..empty_plan()
    }
}

fn default_cursor() -> CursorSnapshot {
    CursorSnapshot {
        row: 0,
        col: 0,
        visible: true,
        shape: CursorShape::Block,
        blinking: true,
    }
}
