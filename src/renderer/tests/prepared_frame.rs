use crate::cell::Style;
use crate::config::{DEFAULT_ANSI_COLORS_RGB8, DEFAULT_DIM_OPACITY};
use crate::renderer::{
    GlyphBitmap, GlyphEntry, PlannedBackground, PlannedGlyph, PreparedSurfaceGlyphFrame,
    RenderPlan, SurfaceFrameError,
};
use crate::terminal::{CursorShape, CursorSnapshot};

#[test]
fn prepared_surface_glyph_frame_rejects_oversized_glyph_bitmap_before_padding() {
    let entry = GlyphEntry {
        slot: 0,
        generation: 0,
    };
    let plan = RenderPlan {
        viewport_cols: 2,
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
            text: "A".to_owned(),
            ch: 'A',
            style: Style::default(),
            font_size_px: 14,
            is_wide: false,
            atlas_entry: entry,
        }],
    };
    let glyphs = [GlyphBitmap {
        entry,
        origin_x: 0,
        origin_y: 0,
        width: u32::MAX,
        height: 1,
        rgba: Vec::new(),
    }];

    let error = PreparedSurfaceGlyphFrame::from_render_plan(
        &plan,
        &glyphs,
        14,
        14,
        [0.0, 0.0, 0.0, 1.0],
        [244, 192, 106, 255],
        0,
    )
    .unwrap_err();

    assert_eq!(
        error,
        SurfaceFrameError::InvalidFrame(
            "glyph slot 0 expected 17179869180 rgba bytes before padding".to_owned()
        )
    );
}

#[test]
fn prepared_surface_glyph_frame_builds_cursor_only_blank_frame() {
    let plan = RenderPlan {
        viewport_cols: 8,
        viewport_rows: 2,
        cursor: CursorSnapshot {
            row: 0,
            col: 0,
            visible: true,
            shape: CursorShape::Block,
            blinking: true,
        },
        default_foreground_rgb8: [232, 226, 214],
        ansi_colors_rgb8: DEFAULT_ANSI_COLORS_RGB8,
        dim_opacity: DEFAULT_DIM_OPACITY,
        clear_regions: Vec::new(),
        backgrounds: Vec::new(),
        decorations: Vec::new(),
        glyphs: Vec::new(),
    };

    let prepared = PreparedSurfaceGlyphFrame::from_render_plan(
        &plan,
        &[],
        18,
        22,
        [0.0, 0.0, 0.0, 1.0],
        [244, 192, 106, 255],
        12,
    )
    .unwrap();
    let frame = prepared.as_surface_glyph_frame();
    assert_eq!(frame.height, (2 * 22) + (2 * 12));

    assert!(frame.batch.quads.is_empty());
    assert_eq!(frame.cursor_batch.quads.len(), 1);
    assert_eq!(frame.cursor_batch.indices.len(), 6);
    assert_eq!(frame.atlas.occupied_slots, 0);
    assert_eq!(frame.atlas.width, 18);
    assert_eq!(frame.atlas.height, 22);
    assert_eq!(frame.width, 168);
    assert!(frame.atlas.rgba.iter().all(|byte| *byte == 0));
}

#[test]
fn prepared_surface_glyph_frame_uses_configured_cell_metrics_for_geometry() {
    let entry = GlyphEntry {
        slot: 0,
        generation: 0,
    };
    let plan = RenderPlan {
        viewport_cols: 3,
        viewport_rows: 1,
        cursor: CursorSnapshot {
            row: 0,
            col: 0,
            visible: false,
            shape: CursorShape::Block,
            blinking: true,
        },
        default_foreground_rgb8: [232, 226, 214],
        ansi_colors_rgb8: DEFAULT_ANSI_COLORS_RGB8,
        dim_opacity: DEFAULT_DIM_OPACITY,
        clear_regions: Vec::new(),
        backgrounds: Vec::new(),
        decorations: Vec::new(),
        glyphs: vec![PlannedGlyph {
            row: 0,
            col: 1,
            text: "i".to_owned(),
            ch: 'i',
            style: Style::default(),
            font_size_px: 18,
            is_wide: false,
            atlas_entry: entry,
        }],
    };
    let glyphs = [GlyphBitmap {
        entry,
        origin_x: 0,
        origin_y: 0,
        width: 5,
        height: 7,
        rgba: vec![255; 5 * 7 * 4],
    }];

    let prepared = PreparedSurfaceGlyphFrame::from_render_plan(
        &plan,
        &glyphs,
        18,
        22,
        [0.0, 0.0, 0.0, 1.0],
        [244, 192, 106, 255],
        4,
    )
    .unwrap();
    let frame = prepared.as_surface_glyph_frame();
    let glyph_quad = &frame.batch.quads[0];

    assert_eq!(frame.width, (3 * 18) + (2 * 4));
    assert_eq!(frame.height, 22 + (2 * 4));
    assert_eq!(frame.atlas.width, 18);
    assert_eq!(frame.atlas.height, 22);
    assert_eq!(glyph_quad.vertices[0].position, [22.0, 4.0]);
    assert_eq!(glyph_quad.vertices[2].position, [40.0, 26.0]);
}

#[test]
fn prepared_surface_glyph_frame_preserves_shaped_glyph_placement_in_atlas() {
    let entry = GlyphEntry {
        slot: 0,
        generation: 0,
    };
    let plan = RenderPlan {
        viewport_cols: 1,
        viewport_rows: 1,
        cursor: CursorSnapshot {
            row: 0,
            col: 0,
            visible: false,
            shape: CursorShape::Block,
            blinking: true,
        },
        default_foreground_rgb8: [232, 226, 214],
        ansi_colors_rgb8: DEFAULT_ANSI_COLORS_RGB8,
        dim_opacity: DEFAULT_DIM_OPACITY,
        clear_regions: Vec::new(),
        backgrounds: Vec::new(),
        decorations: Vec::new(),
        glyphs: vec![PlannedGlyph {
            row: 0,
            col: 0,
            text: "A".to_owned(),
            ch: 'A',
            style: Style::default(),
            font_size_px: 18,
            is_wide: false,
            atlas_entry: entry,
        }],
    };
    let glyphs = [GlyphBitmap {
        entry,
        origin_x: 1,
        origin_y: -2,
        width: 2,
        height: 2,
        rgba: [24, 48, 96, 255].repeat(4),
    }];

    let prepared = PreparedSurfaceGlyphFrame::from_render_plan(
        &plan,
        &glyphs,
        8,
        8,
        [0.0, 0.0, 0.0, 1.0],
        [244, 192, 106, 255],
        0,
    )
    .unwrap();
    let frame = prepared.as_surface_glyph_frame();
    let placed_pixel_offset = ((3 * frame.atlas.width) + 3) as usize * 4;

    assert_eq!(frame.atlas.width, 8);
    assert_eq!(frame.atlas.height, 8);
    assert_eq!(
        &frame.atlas.rgba[placed_pixel_offset..placed_pixel_offset + 4],
        &[24, 48, 96, 255]
    );
    assert!(
        frame.atlas.rgba[0..placed_pixel_offset]
            .iter()
            .all(|byte| *byte == 0)
    );
}

#[test]
fn prepared_surface_glyph_frame_preview_renders_background_glyph_and_cursor_pixels() {
    let entry = GlyphEntry {
        slot: 0,
        generation: 0,
    };
    let plan = RenderPlan {
        viewport_cols: 2,
        viewport_rows: 1,
        cursor: CursorSnapshot {
            row: 0,
            col: 1,
            visible: true,
            shape: CursorShape::Block,
            blinking: true,
        },
        default_foreground_rgb8: [240, 240, 240],
        ansi_colors_rgb8: DEFAULT_ANSI_COLORS_RGB8,
        dim_opacity: DEFAULT_DIM_OPACITY,
        clear_regions: Vec::new(),
        backgrounds: vec![PlannedBackground {
            row: 0,
            col: 0,
            cols: 1,
            color_rgba8: [30, 40, 50, 255],
        }],
        decorations: Vec::new(),
        glyphs: vec![PlannedGlyph {
            row: 0,
            col: 0,
            text: "A".to_owned(),
            ch: 'A',
            style: Style::default(),
            font_size_px: 8,
            is_wide: false,
            atlas_entry: entry,
        }],
    };
    let glyphs = [GlyphBitmap {
        entry,
        origin_x: 0,
        origin_y: 0,
        width: 2,
        height: 2,
        rgba: vec![255; 2 * 2 * 4],
    }];

    let prepared = PreparedSurfaceGlyphFrame::from_render_plan(
        &plan,
        &glyphs,
        2,
        2,
        [0.0, 0.0, 0.0, 1.0],
        [244, 192, 106, 255],
        0,
    )
    .unwrap();

    let preview = prepared.preview_rgba8().unwrap();

    assert_eq!(preview.width, 4);
    assert_eq!(preview.height, 2);
    assert_eq!(preview.rgba.len(), 4 * 2 * 4);
    assert_eq!(
        preview_pixel(&preview.rgba, preview.width, 0, 0),
        [240, 240, 240, 255]
    );
    assert_eq!(
        preview_pixel(&preview.rgba, preview.width, 2, 0),
        [244, 192, 106, 255]
    );
}

fn preview_pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * width + x) * 4) as usize;
    [
        rgba[offset],
        rgba[offset + 1],
        rgba[offset + 2],
        rgba[offset + 3],
    ]
}
