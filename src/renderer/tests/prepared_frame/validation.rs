use crate::cell::Style;
use crate::config::{DEFAULT_ANSI_COLORS_RGB8, DEFAULT_DIM_OPACITY};
use crate::renderer::{
    GlyphBitmap, GlyphEntry, PlannedGlyph, PreparedSurfaceGlyphFrame, RenderPlan, SurfaceFrameError,
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
