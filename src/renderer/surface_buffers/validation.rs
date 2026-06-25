use super::super::SurfaceGlyphFrame;
use super::super::surface_frame::SurfaceFrameError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::renderer) struct SurfaceGlyphAtlasLayout {
    pub(in crate::renderer) row_bytes: u32,
    pub(in crate::renderer) expected_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::renderer) struct SurfaceGlyphBufferLayout {
    pub(in crate::renderer) vertex_buffer_size: u64,
    pub(in crate::renderer) index_buffer_size: u64,
    pub(in crate::renderer) index_count: u32,
}

pub(in crate::renderer) fn validate_surface_glyph_frame(
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
    validate_quad_index_presence(frame)?;
    Ok(SurfaceGlyphAtlasLayout {
        row_bytes,
        expected_len,
    })
}

pub(in crate::renderer) fn validate_surface_background_buffers(
    vertex_bytes: &[u8],
    index_bytes: &[u8],
    index_count: usize,
) -> std::result::Result<SurfaceGlyphBufferLayout, SurfaceFrameError> {
    validate_surface_buffers(vertex_bytes, index_bytes, index_count, "surface background")
}

pub(in crate::renderer) fn validate_surface_glyph_buffers(
    vertex_bytes: &[u8],
    index_bytes: &[u8],
    index_count: usize,
) -> std::result::Result<SurfaceGlyphBufferLayout, SurfaceFrameError> {
    validate_surface_buffers(vertex_bytes, index_bytes, index_count, "surface glyph")
}

fn validate_quad_index_presence(
    frame: SurfaceGlyphFrame<'_>,
) -> std::result::Result<(), SurfaceFrameError> {
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
    Ok(())
}

fn validate_surface_buffers(
    vertex_bytes: &[u8],
    index_bytes: &[u8],
    index_count: usize,
    label: &str,
) -> std::result::Result<SurfaceGlyphBufferLayout, SurfaceFrameError> {
    if vertex_bytes.is_empty() || index_bytes.is_empty() || index_count == 0 {
        return Err(SurfaceFrameError::InvalidFrame(format!(
            "{label} draw buffers must be non-empty"
        )));
    }
    let vertex_buffer_size = u64::try_from(vertex_bytes.len()).map_err(|_| {
        SurfaceFrameError::InvalidFrame(format!("{label} vertex buffer is too large"))
    })?;
    let index_buffer_size = u64::try_from(index_bytes.len()).map_err(|_| {
        SurfaceFrameError::InvalidFrame(format!("{label} index buffer is too large"))
    })?;
    let index_count = u32::try_from(index_count).map_err(|_| {
        SurfaceFrameError::InvalidFrame(format!("{label} index count is too large"))
    })?;
    Ok(SurfaceGlyphBufferLayout {
        vertex_buffer_size,
        index_buffer_size,
        index_count,
    })
}
