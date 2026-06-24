use super::GpuBootstrapError;
use crate::renderer::{BackgroundQuadBatch, GlyphQuadBatch};

pub(super) fn textured_quad_vertex_bytes() -> Vec<u8> {
    [
        [-1.0_f32, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0],
        [1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        [-1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0],
    ]
    .into_iter()
    .flat_map(|vertex| vertex.into_iter().flat_map(f32::to_le_bytes))
    .collect()
}

pub(super) fn textured_quad_index_bytes() -> Vec<u8> {
    [0_u16, 1, 2, 0, 2, 3]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect()
}

pub(super) fn background_quad_vertex_bytes(
    batch: &BackgroundQuadBatch,
    width: u32,
    height: u32,
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    if width == 0 || height == 0 {
        return Err(GpuBootstrapError::SmokeReadback(
            "terminal text render target dimensions must be non-zero".to_owned(),
        ));
    }
    let width = width as f32;
    let height = height as f32;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(background_quad_vertex_byte_capacity(batch.quads.len())?)
        .map_err(|_| {
            GpuBootstrapError::SmokeReadback(
                "terminal background vertex bytes are too large to allocate".to_owned(),
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

fn background_quad_vertex_byte_capacity(
    quad_count: usize,
) -> std::result::Result<usize, GpuBootstrapError> {
    quad_count.checked_mul(4 * 6 * 4).ok_or_else(|| {
        GpuBootstrapError::SmokeReadback(
            "terminal background vertex bytes are too large".to_owned(),
        )
    })
}

pub(super) fn background_quad_index_bytes(batch: &BackgroundQuadBatch) -> Vec<u8> {
    batch
        .indices
        .iter()
        .flat_map(|index| index.to_le_bytes())
        .collect()
}

pub(super) fn glyph_quad_vertex_bytes(
    batch: &GlyphQuadBatch,
    width: u32,
    height: u32,
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    if width == 0 || height == 0 {
        return Err(GpuBootstrapError::SmokeReadback(
            "terminal text render target dimensions must be non-zero".to_owned(),
        ));
    }
    let width = width as f32;
    let height = height as f32;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(glyph_quad_vertex_byte_capacity(batch.quads.len())?)
        .map_err(|_| {
            GpuBootstrapError::SmokeReadback(
                "terminal text vertex bytes are too large to allocate".to_owned(),
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

fn glyph_quad_vertex_byte_capacity(
    quad_count: usize,
) -> std::result::Result<usize, GpuBootstrapError> {
    quad_count.checked_mul(4 * 8 * 4).ok_or_else(|| {
        GpuBootstrapError::SmokeReadback("terminal text vertex bytes are too large".to_owned())
    })
}

pub(super) fn glyph_quad_index_bytes(batch: &GlyphQuadBatch) -> Vec<u8> {
    batch
        .indices
        .iter()
        .flat_map(|index| index.to_le_bytes())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_quad_vertex_byte_capacity_uses_checked_multiplication() {
        assert_eq!(glyph_quad_vertex_byte_capacity(2).unwrap(), 256);

        let error = glyph_quad_vertex_byte_capacity((usize::MAX / 128) + 1).unwrap_err();

        assert_eq!(
            error,
            GpuBootstrapError::SmokeReadback("terminal text vertex bytes are too large".to_owned())
        );
    }

    #[test]
    fn background_quad_vertex_byte_capacity_uses_checked_multiplication() {
        assert_eq!(background_quad_vertex_byte_capacity(2).unwrap(), 192);

        let error = background_quad_vertex_byte_capacity((usize::MAX / 96) + 1).unwrap_err();

        assert_eq!(
            error,
            GpuBootstrapError::SmokeReadback(
                "terminal background vertex bytes are too large".to_owned()
            )
        );
    }
}
