use crate::cell::Style;
use crate::config::{
    DEFAULT_BACKGROUND_RGB8, DEFAULT_CURSOR_RGB8, DEFAULT_FOREGROUND_RGB8,
    DEFAULT_SURFACE_PADDING_PX,
};
use crate::renderer::{
    GlyphBitmap, GlyphEntry, PlannedGlyph, PreparedSurfaceGlyphFrame, RenderPlan, RendererConfig,
};
use crate::terminal::{CursorShape, CursorSnapshot};

use super::support::{preview_pixel, rgb};

#[test]
fn default_theme_prepared_frame_preview_keeps_text_padded_legible_and_unclipped() {
    let renderer_config = RendererConfig::default();
    let entry = GlyphEntry {
        slot: 0,
        generation: 0,
    };
    let plan = RenderPlan {
        viewport_cols: 3,
        viewport_rows: 1,
        cursor: CursorSnapshot {
            row: 0,
            col: 1,
            visible: true,
            shape: CursorShape::Block,
            blinking: true,
        },
        default_foreground_rgb8: renderer_config.default_foreground_rgb8,
        ansi_colors_rgb8: renderer_config.ansi_colors_rgb8,
        dim_opacity: renderer_config.dim_opacity,
        clear_regions: Vec::new(),
        backgrounds: Vec::new(),
        decorations: Vec::new(),
        glyphs: vec![PlannedGlyph {
            row: 0,
            col: 0,
            text: "G".to_owned(),
            ch: 'G',
            style: Style::default(),
            font_size_px: renderer_config.font_size_px,
            is_wide: false,
            atlas_entry: entry,
        }],
    };
    let cell_width = u32::from(renderer_config.cell_width_px);
    let line_height = u32::from(renderer_config.line_height_px);
    let padding = u32::from(renderer_config.surface_padding_px);
    let glyphs = [GlyphBitmap {
        entry,
        origin_x: 0,
        origin_y: 0,
        width: cell_width,
        height: line_height,
        rgba: vec![255; (cell_width * line_height * 4) as usize],
    }];

    let prepared = PreparedSurfaceGlyphFrame::from_render_plan(
        &plan,
        &glyphs,
        renderer_config.cell_width_px,
        renderer_config.line_height_px,
        renderer_config.clear_color,
        renderer_config.cursor_color_rgba8,
        renderer_config.surface_padding_px,
    )
    .unwrap();
    let preview = prepared.preview_rgba8().unwrap();

    assert_eq!(renderer_config.font_size_px, 37);
    assert_eq!(renderer_config.cell_width_px, 21);
    assert_eq!(renderer_config.line_height_px, 51);
    assert_eq!(
        renderer_config.surface_padding_px,
        DEFAULT_SURFACE_PADDING_PX
    );
    assert_eq!(
        preview.width,
        (3 * u32::from(renderer_config.cell_width_px))
            + (2 * u32::from(renderer_config.surface_padding_px))
    );
    assert_eq!(
        preview.height,
        u32::from(renderer_config.line_height_px)
            + (2 * u32::from(renderer_config.surface_padding_px))
    );
    assert_eq!(
        preview_pixel(&preview.rgba, preview.width, 0, 0),
        rgb(DEFAULT_BACKGROUND_RGB8)
    );
    assert_eq!(
        preview_pixel(&preview.rgba, preview.width, padding - 1, padding),
        rgb(DEFAULT_BACKGROUND_RGB8)
    );
    assert_eq!(
        preview_pixel(&preview.rgba, preview.width, padding, padding),
        rgb(DEFAULT_FOREGROUND_RGB8)
    );
    assert_eq!(
        preview_pixel(
            &preview.rgba,
            preview.width,
            padding + cell_width - 1,
            padding + line_height - 1,
        ),
        rgb(DEFAULT_FOREGROUND_RGB8)
    );
    assert_eq!(
        preview_pixel(&preview.rgba, preview.width, padding + cell_width, padding),
        rgb(DEFAULT_CURSOR_RGB8)
    );
    assert_eq!(
        preview_pixel(
            &preview.rgba,
            preview.width,
            preview.width - 1,
            preview.height - 1,
        ),
        rgb(DEFAULT_BACKGROUND_RGB8)
    );
}
