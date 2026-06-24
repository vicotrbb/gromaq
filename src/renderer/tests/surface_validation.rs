use crate::renderer::surface_buffers::{
    SurfaceGlyphAtlasLayout, SurfaceGlyphBufferLayout, surface_background_vertex_byte_capacity,
    surface_glyph_vertex_byte_capacity, validate_surface_background_buffers,
    validate_surface_glyph_buffers, validate_surface_glyph_frame,
};
use crate::renderer::{
    BackgroundQuad, BackgroundQuadBatch, BackgroundVertex, GlyphAtlasImage, GlyphEntry, GlyphQuad,
    GlyphQuadBatch, GlyphVertex, SurfaceFrameError, SurfaceGlyphFrame,
};

fn one_quad_batch() -> GlyphQuadBatch {
    let quad = GlyphQuad {
        text: "A".to_owned(),
        ch: 'A',
        atlas_entry: GlyphEntry {
            slot: 0,
            generation: 0,
        },
        vertices: [
            GlyphVertex {
                position: [0.0, 0.0],
                uv: [0.0, 0.0],
                foreground_rgba: [1.0, 1.0, 1.0, 1.0],
            },
            GlyphVertex {
                position: [1.0, 0.0],
                uv: [1.0, 0.0],
                foreground_rgba: [1.0, 1.0, 1.0, 1.0],
            },
            GlyphVertex {
                position: [1.0, 1.0],
                uv: [1.0, 1.0],
                foreground_rgba: [1.0, 1.0, 1.0, 1.0],
            },
            GlyphVertex {
                position: [0.0, 1.0],
                uv: [0.0, 1.0],
                foreground_rgba: [1.0, 1.0, 1.0, 1.0],
            },
        ],
    };
    GlyphQuadBatch {
        quads: vec![quad],
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}

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
    let background_batch = BackgroundQuadBatch {
        quads: vec![BackgroundQuad {
            row: 0,
            col: 0,
            cols: 1,
            vertices: [
                BackgroundVertex {
                    position: [0.0, 0.0],
                    color_rgba: [1.0, 0.0, 0.0, 1.0],
                },
                BackgroundVertex {
                    position: [1.0, 0.0],
                    color_rgba: [1.0, 0.0, 0.0, 1.0],
                },
                BackgroundVertex {
                    position: [1.0, 1.0],
                    color_rgba: [1.0, 0.0, 0.0, 1.0],
                },
                BackgroundVertex {
                    position: [0.0, 1.0],
                    color_rgba: [1.0, 0.0, 0.0, 1.0],
                },
            ],
        }],
        indices: vec![0, 1, 2, 0, 2, 3],
    };

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
    let cursor_batch = BackgroundQuadBatch {
        quads: vec![BackgroundQuad {
            row: 0,
            col: 0,
            cols: 1,
            vertices: [
                BackgroundVertex {
                    position: [0.0, 0.0],
                    color_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                BackgroundVertex {
                    position: [1.0, 0.0],
                    color_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                BackgroundVertex {
                    position: [1.0, 1.0],
                    color_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                BackgroundVertex {
                    position: [0.0, 1.0],
                    color_rgba: [1.0, 1.0, 1.0, 1.0],
                },
            ],
        }],
        indices: vec![0, 1, 2, 0, 2, 3],
    };

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

#[test]
fn surface_glyph_buffer_validation_reports_checked_sizes() {
    let vertex_bytes = [1_u8, 2, 3, 4];
    let index_bytes = [5_u8, 6, 7, 8];

    let layout = validate_surface_glyph_buffers(&vertex_bytes, &index_bytes, 1).unwrap();

    assert_eq!(
        layout,
        SurfaceGlyphBufferLayout {
            vertex_buffer_size: 4,
            index_buffer_size: 4,
            index_count: 1,
        }
    );
}

#[test]
fn surface_background_buffer_validation_reports_checked_sizes() {
    let vertex_bytes = [1_u8, 2, 3, 4];
    let index_bytes = [5_u8, 6, 7, 8];

    let layout = validate_surface_background_buffers(&vertex_bytes, &index_bytes, 1).unwrap();

    assert_eq!(
        layout,
        SurfaceGlyphBufferLayout {
            vertex_buffer_size: 4,
            index_buffer_size: 4,
            index_count: 1,
        }
    );
}

#[test]
fn surface_glyph_vertex_byte_capacity_uses_checked_multiplication() {
    assert_eq!(surface_glyph_vertex_byte_capacity(2).unwrap(), 256);

    let error = surface_glyph_vertex_byte_capacity((usize::MAX / 128) + 1).unwrap_err();

    assert_eq!(
        error,
        SurfaceFrameError::InvalidFrame("surface glyph vertex bytes are too large".to_owned())
    );
}

#[test]
fn surface_background_vertex_byte_capacity_uses_checked_multiplication() {
    assert_eq!(surface_background_vertex_byte_capacity(2).unwrap(), 192);

    let error = surface_background_vertex_byte_capacity((usize::MAX / 96) + 1).unwrap_err();

    assert_eq!(
        error,
        SurfaceFrameError::InvalidFrame("surface background vertex bytes are too large".to_owned())
    );
}

#[test]
fn surface_glyph_buffer_validation_rejects_empty_buffers() {
    let vertex_bytes = [];
    let index_bytes = [1_u8, 2, 3, 4];

    let error = validate_surface_glyph_buffers(&vertex_bytes, &index_bytes, 1).unwrap_err();

    assert_eq!(
        error,
        SurfaceFrameError::InvalidFrame("surface glyph draw buffers must be non-empty".to_owned())
    );
}

#[test]
fn surface_background_buffer_validation_rejects_empty_buffers() {
    let vertex_bytes = [];
    let index_bytes = [1_u8, 2, 3, 4];

    let error = validate_surface_background_buffers(&vertex_bytes, &index_bytes, 1).unwrap_err();

    assert_eq!(
        error,
        SurfaceFrameError::InvalidFrame(
            "surface background draw buffers must be non-empty".to_owned()
        )
    );
}

#[test]
#[cfg(target_pointer_width = "64")]
fn surface_glyph_buffer_validation_rejects_oversized_index_count() {
    let vertex_bytes = [1_u8, 2, 3, 4];
    let index_bytes = [5_u8, 6, 7, 8];

    let error =
        validate_surface_glyph_buffers(&vertex_bytes, &index_bytes, usize::MAX).unwrap_err();

    assert_eq!(
        error,
        SurfaceFrameError::InvalidFrame("surface glyph index count is too large".to_owned())
    );
}
