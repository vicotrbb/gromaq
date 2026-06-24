use super::{BackgroundQuadBatch, GlyphQuadBatch, SurfaceFrameError, SurfaceGlyphFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SurfaceGlyphAtlasLayout {
    pub(super) row_bytes: u32,
    pub(super) expected_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SurfaceGlyphBufferLayout {
    pub(super) vertex_buffer_size: u64,
    pub(super) index_buffer_size: u64,
    pub(super) index_count: u32,
}

pub(super) fn validate_surface_glyph_frame(
    frame: SurfaceGlyphFrame<'_>,
) -> std::result::Result<SurfaceGlyphAtlasLayout, SurfaceFrameError> {
    if frame.width == 0 || frame.height == 0 {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph frame dimensions must be non-zero".to_owned(),
        ));
    }
    if frame.atlas.width == 0 || frame.atlas.height == 0 {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph atlas dimensions must be non-zero".to_owned(),
        ));
    }
    let row_bytes = frame.atlas.width.checked_mul(4).ok_or_else(|| {
        SurfaceFrameError::InvalidFrame("surface glyph atlas row size is too large".to_owned())
    })?;
    let expected_len = usize::try_from(row_bytes)
        .ok()
        .and_then(|row_bytes| {
            usize::try_from(frame.atlas.height)
                .ok()
                .and_then(|height| row_bytes.checked_mul(height))
        })
        .ok_or_else(|| {
            SurfaceFrameError::InvalidFrame("surface glyph atlas byte size is too large".to_owned())
        })?;
    if frame.atlas.rgba.len() != expected_len {
        return Err(SurfaceFrameError::InvalidFrame(format!(
            "surface glyph atlas has {} bytes, expected {expected_len}",
            frame.atlas.rgba.len()
        )));
    }
    if frame.batch.quads.is_empty() != frame.batch.indices.is_empty() {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph quads and indices must both be present or both be empty".to_owned(),
        ));
    }
    if frame.background_batch.quads.is_empty() != frame.background_batch.indices.is_empty() {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface background quads and indices must both be present or both be empty".to_owned(),
        ));
    }
    if frame.decoration_batch.quads.is_empty() != frame.decoration_batch.indices.is_empty() {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface decoration quads and indices must both be present or both be empty".to_owned(),
        ));
    }
    if frame.cursor_batch.quads.is_empty() != frame.cursor_batch.indices.is_empty() {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface cursor quads and indices must both be present or both be empty".to_owned(),
        ));
    }
    if frame.batch.quads.is_empty()
        && frame.background_batch.quads.is_empty()
        && frame.decoration_batch.quads.is_empty()
        && frame.cursor_batch.quads.is_empty()
    {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph frame requires non-empty glyph, background, decoration, or cursor quads"
                .to_owned(),
        ));
    }
    Ok(SurfaceGlyphAtlasLayout {
        row_bytes,
        expected_len,
    })
}

pub(super) fn validate_surface_background_buffers(
    vertex_bytes: &[u8],
    index_bytes: &[u8],
    index_count: usize,
) -> std::result::Result<SurfaceGlyphBufferLayout, SurfaceFrameError> {
    if vertex_bytes.is_empty() || index_bytes.is_empty() || index_count == 0 {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface background draw buffers must be non-empty".to_owned(),
        ));
    }
    let vertex_buffer_size = u64::try_from(vertex_bytes.len()).map_err(|_| {
        SurfaceFrameError::InvalidFrame("surface background vertex buffer is too large".to_owned())
    })?;
    let index_buffer_size = u64::try_from(index_bytes.len()).map_err(|_| {
        SurfaceFrameError::InvalidFrame("surface background index buffer is too large".to_owned())
    })?;
    let index_count = u32::try_from(index_count).map_err(|_| {
        SurfaceFrameError::InvalidFrame("surface background index count is too large".to_owned())
    })?;
    Ok(SurfaceGlyphBufferLayout {
        vertex_buffer_size,
        index_buffer_size,
        index_count,
    })
}

pub(super) fn validate_surface_glyph_buffers(
    vertex_bytes: &[u8],
    index_bytes: &[u8],
    index_count: usize,
) -> std::result::Result<SurfaceGlyphBufferLayout, SurfaceFrameError> {
    if vertex_bytes.is_empty() || index_bytes.is_empty() || index_count == 0 {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph draw buffers must be non-empty".to_owned(),
        ));
    }
    let vertex_buffer_size = u64::try_from(vertex_bytes.len()).map_err(|_| {
        SurfaceFrameError::InvalidFrame("surface glyph vertex buffer is too large".to_owned())
    })?;
    let index_buffer_size = u64::try_from(index_bytes.len()).map_err(|_| {
        SurfaceFrameError::InvalidFrame("surface glyph index buffer is too large".to_owned())
    })?;
    let index_count = u32::try_from(index_count).map_err(|_| {
        SurfaceFrameError::InvalidFrame("surface glyph index count is too large".to_owned())
    })?;
    Ok(SurfaceGlyphBufferLayout {
        vertex_buffer_size,
        index_buffer_size,
        index_count,
    })
}

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
