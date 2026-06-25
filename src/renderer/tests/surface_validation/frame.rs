use super::{one_background_batch, one_quad_batch};
use crate::renderer::surface_buffers::{SurfaceGlyphAtlasLayout, validate_surface_glyph_frame};
use crate::renderer::{
    BackgroundQuadBatch, GlyphAtlasImage, GlyphQuadBatch, SurfaceFrameError, SurfaceGlyphFrame,
};

#[test]
fn surface_glyph_frame_validation_computes_checked_atlas_layout() {
    let atlas = GlyphAtlasImage {
        width: 2,
        height: 2,
        rgba: vec![255; 16],
        occupied_slots: 1,
    };
    let batch = one_quad_batch();

    let layout = validate_surface_glyph_frame(SurfaceGlyphFrame {
        atlas: &atlas,
        background_batch: &BackgroundQuadBatch::default(),
        batch: &batch,
        decoration_batch: &BackgroundQuadBatch::default(),
        cursor_batch: &BackgroundQuadBatch::default(),
        width: 16,
        height: 16,
        clear_color: [0.0, 0.0, 0.0, 1.0],
    })
    .unwrap();

    assert_eq!(
        layout,
        SurfaceGlyphAtlasLayout {
            row_bytes: 8,
            expected_len: 16,
        }
    );
}

#[test]
fn surface_glyph_frame_validation_rejects_overflowing_atlas_row_size() {
    let atlas = GlyphAtlasImage {
        width: u32::MAX,
        height: 1,
        rgba: Vec::new(),
        occupied_slots: 0,
    };
    let batch = one_quad_batch();

    let error = validate_surface_glyph_frame(SurfaceGlyphFrame {
        atlas: &atlas,
        background_batch: &BackgroundQuadBatch::default(),
        batch: &batch,
        decoration_batch: &BackgroundQuadBatch::default(),
        cursor_batch: &BackgroundQuadBatch::default(),
        width: 16,
        height: 16,
        clear_color: [0.0, 0.0, 0.0, 1.0],
    })
    .unwrap_err();

    assert_eq!(
        error,
        SurfaceFrameError::InvalidFrame("surface glyph atlas row size is too large".to_owned())
    );
}

#[test]
fn surface_glyph_frame_validation_accepts_background_only_batches() {
    let atlas = GlyphAtlasImage {
        width: 1,
        height: 1,
        rgba: vec![0; 4],
        occupied_slots: 0,
    };
    let background_batch = one_background_batch([1.0, 0.0, 0.0, 1.0]);

    let layout = validate_surface_glyph_frame(SurfaceGlyphFrame {
        atlas: &atlas,
        background_batch: &background_batch,
        batch: &GlyphQuadBatch::default(),
        decoration_batch: &BackgroundQuadBatch::default(),
        cursor_batch: &BackgroundQuadBatch::default(),
        width: 1,
        height: 1,
        clear_color: [0.0, 0.0, 0.0, 1.0],
    })
    .unwrap();

    assert_eq!(
        layout,
        SurfaceGlyphAtlasLayout {
            row_bytes: 4,
            expected_len: 4,
        }
    );
}

#[test]
fn surface_glyph_frame_validation_accepts_cursor_only_batches() {
    let atlas = GlyphAtlasImage {
        width: 1,
        height: 1,
        rgba: vec![0; 4],
        occupied_slots: 0,
    };
    let cursor_batch = one_background_batch([1.0, 1.0, 1.0, 1.0]);

    let layout = validate_surface_glyph_frame(SurfaceGlyphFrame {
        atlas: &atlas,
        background_batch: &BackgroundQuadBatch::default(),
        batch: &GlyphQuadBatch::default(),
        decoration_batch: &BackgroundQuadBatch::default(),
        cursor_batch: &cursor_batch,
        width: 1,
        height: 1,
        clear_color: [0.0, 0.0, 0.0, 1.0],
    })
    .unwrap();

    assert_eq!(
        layout,
        SurfaceGlyphAtlasLayout {
            row_bytes: 4,
            expected_len: 4,
        }
    );
}
