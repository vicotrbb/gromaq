use crate::cell::Style;
use crate::config::{DEFAULT_ANSI_COLORS_RGB8, DEFAULT_DIM_OPACITY};
use crate::renderer::{
    GlyphBitmap, GlyphEntry, PlannedBackground, PlannedGlyph, PreparedSurfaceGlyphFrame, RenderPlan,
};
use crate::terminal::{CursorShape, CursorSnapshot};

use super::support::preview_pixel;

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
