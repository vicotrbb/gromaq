use crate::cell::{Color, Style};
use crate::config::{DEFAULT_ANSI_COLORS_RGB8, DEFAULT_DIM_OPACITY};
use crate::renderer::{
    GlyphBitmap, GlyphEntry, PlannedBackground, PlannedGlyph, PreparedSurfaceGlyphFrame,
    PreparedSurfaceGlyphFrameConfig, RenderPlan,
};
use crate::terminal::{CursorShape, CursorSnapshot};

use super::support::rgba;

#[test]
fn prepared_surface_glyph_frame_carries_themed_plan_colors_into_batches() {
    let entry = GlyphEntry {
        slot: 0,
        generation: 0,
    };
    let mut ansi_colors_rgb8 = DEFAULT_ANSI_COLORS_RGB8;
    ansi_colors_rgb8[4] = [138, 180, 255];
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
        default_foreground_rgb8: [243, 246, 251],
        ansi_colors_rgb8,
        dim_opacity: DEFAULT_DIM_OPACITY,
        clear_regions: Vec::new(),
        backgrounds: vec![PlannedBackground {
            row: 0,
            col: 0,
            cols: 1,
            color_rgba8: [38, 68, 95, 255],
        }],
        decorations: Vec::new(),
        glyphs: vec![PlannedGlyph {
            row: 0,
            col: 0,
            text: "G".to_owned(),
            ch: 'G',
            style: Style {
                foreground: Color::Ansi(4),
                ..Style::default()
            },
            font_size_px: 24,
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
        PreparedSurfaceGlyphFrameConfig {
            cell_width_px: 13,
            line_height_px: 33,
            clear_color: [0.0, 0.0, 0.0, 1.0],
            cursor_color_rgba8: [255, 209, 102, 255],
            surface_padding_px: 14,
            cell_spacing_px: 0,
        },
    )
    .unwrap();

    assert_eq!(prepared.background_batch().quads.len(), 1);
    assert_eq!(
        prepared.background_batch().quads[0].vertices[0].color_rgba,
        rgba(38, 68, 95, 1.0)
    );
    assert_eq!(prepared.batch().quads.len(), 1);
    assert_eq!(
        prepared.batch().quads[0].vertices[0].foreground_rgba,
        rgba(138, 180, 255, 1.0)
    );
    assert_eq!(prepared.cursor_batch().quads.len(), 1);
    assert_eq!(
        prepared.cursor_batch().quads[0].vertices[0].color_rgba,
        rgba(255, 209, 102, 1.0)
    );
}
