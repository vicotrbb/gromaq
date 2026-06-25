use super::{one_background_batch, one_quad_batch};
use crate::renderer::SurfaceFrameError;
use crate::renderer::surface_buffers::{
    SurfaceGlyphBufferLayout, surface_background_vertex_byte_capacity,
    surface_background_vertex_bytes, surface_glyph_vertex_byte_capacity,
    surface_glyph_vertex_bytes, validate_surface_background_buffers,
    validate_surface_glyph_buffers,
};

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
fn surface_glyph_vertices_normalize_against_surface_target_size() {
    let mut batch = one_quad_batch();
    batch.quads[0].vertices[0].position = [16.0, 22.0];

    let vertex_bytes = surface_glyph_vertex_bytes(&batch, 1280, 800).unwrap();

    assert_f32_near(f32_at(&vertex_bytes, 0), -0.975);
    assert_f32_near(f32_at(&vertex_bytes, 4), 0.945);
}

#[test]
fn surface_background_vertices_normalize_against_surface_target_size() {
    let mut batch = one_background_batch([1.0, 0.0, 0.0, 1.0]);
    batch.quads[0].vertices[0].position = [16.0, 22.0];
    batch.quads[0].vertices[1].position = [32.0, 22.0];
    batch.quads[0].vertices[2].position = [32.0, 44.0];
    batch.quads[0].vertices[3].position = [16.0, 44.0];

    let vertex_bytes = surface_background_vertex_bytes(&batch, 1280, 800).unwrap();

    assert_f32_near(f32_at(&vertex_bytes, 0), -0.975);
    assert_f32_near(f32_at(&vertex_bytes, 4), 0.945);
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

fn f32_at(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn assert_f32_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < f32::EPSILON,
        "expected {actual} to equal {expected}"
    );
}
