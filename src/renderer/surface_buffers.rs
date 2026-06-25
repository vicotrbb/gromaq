use super::surface_frame::SurfaceFrameError;
use super::{BackgroundQuadBatch, GlyphQuadBatch};

mod validation;

#[cfg(test)]
pub(super) use validation::SurfaceGlyphBufferLayout;
pub(super) use validation::{
    SurfaceGlyphAtlasLayout, validate_surface_background_buffers, validate_surface_glyph_buffers,
    validate_surface_glyph_frame,
};

pub(super) fn surface_background_vertex_bytes(
    batch: &BackgroundQuadBatch,
    width: u32,
    height: u32,
) -> std::result::Result<Vec<u8>, SurfaceFrameError> {
    if width == 0 || height == 0 {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph frame dimensions must be non-zero".to_owned(),
        ));
    }
    let width = width as f32;
    let height = height as f32;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(surface_background_vertex_byte_capacity(batch.quads.len())?)
        .map_err(|_| {
            SurfaceFrameError::InvalidFrame(
                "surface background vertex bytes are too large to allocate".to_owned(),
            )
        })?;
    for quad in &batch.quads {
        for vertex in quad.vertices {
            let ndc_x = (vertex.position[0] / width * 2.0) - 1.0;
            let ndc_y = 1.0 - (vertex.position[1] / height * 2.0);
            for value in [
                ndc_x,
                ndc_y,
                vertex.color_rgba[0],
                vertex.color_rgba[1],
                vertex.color_rgba[2],
                vertex.color_rgba[3],
            ] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
    Ok(bytes)
}

pub(super) fn surface_background_vertex_byte_capacity(
    quad_count: usize,
) -> std::result::Result<usize, SurfaceFrameError> {
    quad_count.checked_mul(4 * 6 * 4).ok_or_else(|| {
        SurfaceFrameError::InvalidFrame("surface background vertex bytes are too large".to_owned())
    })
}

pub(super) fn surface_background_index_bytes(batch: &BackgroundQuadBatch) -> Vec<u8> {
    batch
        .indices
        .iter()
        .flat_map(|index| index.to_le_bytes())
        .collect()
}

pub(super) fn surface_glyph_vertex_bytes(
    batch: &GlyphQuadBatch,
    width: u32,
    height: u32,
) -> std::result::Result<Vec<u8>, SurfaceFrameError> {
    if width == 0 || height == 0 {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph frame dimensions must be non-zero".to_owned(),
        ));
    }
    let width = width as f32;
    let height = height as f32;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(surface_glyph_vertex_byte_capacity(batch.quads.len())?)
        .map_err(|_| {
            SurfaceFrameError::InvalidFrame(
                "surface glyph vertex bytes are too large to allocate".to_owned(),
            )
        })?;
    for quad in &batch.quads {
        for vertex in quad.vertices {
            let ndc_x = (vertex.position[0] / width * 2.0) - 1.0;
            let ndc_y = 1.0 - (vertex.position[1] / height * 2.0);
            for value in [
                ndc_x,
                ndc_y,
                vertex.uv[0],
                vertex.uv[1],
                vertex.foreground_rgba[0],
                vertex.foreground_rgba[1],
                vertex.foreground_rgba[2],
                vertex.foreground_rgba[3],
            ] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
    Ok(bytes)
}

pub(super) fn surface_glyph_vertex_byte_capacity(
    quad_count: usize,
) -> std::result::Result<usize, SurfaceFrameError> {
    quad_count.checked_mul(4 * 8 * 4).ok_or_else(|| {
        SurfaceFrameError::InvalidFrame("surface glyph vertex bytes are too large".to_owned())
    })
}

pub(super) fn surface_glyph_index_bytes(batch: &GlyphQuadBatch) -> Vec<u8> {
    batch
        .indices
        .iter()
        .flat_map(|index| index.to_le_bytes())
        .collect()
}
