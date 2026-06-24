//! Checked draw-buffer sizing for native GPU smoke render paths.

use super::GpuBootstrapError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DrawBufferLayout {
    pub(super) vertex_buffer_size: u64,
    pub(super) index_buffer_size: u64,
    pub(super) index_count: u32,
}

pub(super) fn checked_textured_index_count(
    index_count: usize,
) -> std::result::Result<u32, GpuBootstrapError> {
    u32::try_from(index_count).map_err(|_| {
        GpuBootstrapError::SmokeReadback("terminal text index count is too large".to_owned())
    })
}

pub(super) fn validate_textured_draw_buffers(
    vertex_bytes: &[u8],
    index_bytes: &[u8],
    index_count: u32,
) -> std::result::Result<DrawBufferLayout, GpuBootstrapError> {
    validate_draw_buffers(
        "textured",
        vertex_bytes,
        index_bytes,
        index_count,
        "textured draw buffers must be non-empty",
    )
}

pub(super) fn validate_background_draw_buffers(
    vertex_bytes: &[u8],
    index_bytes: &[u8],
    index_count: u32,
) -> std::result::Result<DrawBufferLayout, GpuBootstrapError> {
    validate_draw_buffers(
        "background",
        vertex_bytes,
        index_bytes,
        index_count,
        "background draw buffers must be non-empty",
    )
}

fn validate_draw_buffers(
    label: &str,
    vertex_bytes: &[u8],
    index_bytes: &[u8],
    index_count: u32,
    empty_error: &str,
) -> std::result::Result<DrawBufferLayout, GpuBootstrapError> {
    if vertex_bytes.is_empty() || index_bytes.is_empty() || index_count == 0 {
        return Err(GpuBootstrapError::SmokeReadback(empty_error.to_owned()));
    }
    let vertex_buffer_size = u64::try_from(vertex_bytes.len()).map_err(|_| {
        GpuBootstrapError::SmokeReadback(format!("{label} vertex buffer is too large"))
    })?;
    let index_buffer_size = u64::try_from(index_bytes.len()).map_err(|_| {
        GpuBootstrapError::SmokeReadback(format!("{label} index buffer is too large"))
    })?;
    Ok(DrawBufferLayout {
        vertex_buffer_size,
        index_buffer_size,
        index_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textured_draw_buffer_layout_reports_checked_sizes() {
        let vertex_bytes = [1_u8, 2, 3, 4];
        let index_bytes = [5_u8, 6];

        let layout = validate_textured_draw_buffers(&vertex_bytes, &index_bytes, 1).unwrap();

        assert_eq!(
            layout,
            DrawBufferLayout {
                vertex_buffer_size: 4,
                index_buffer_size: 2,
                index_count: 1,
            }
        );
    }

    #[test]
    fn textured_draw_buffer_layout_rejects_empty_buffers() {
        let vertex_bytes = [];
        let index_bytes = [1_u8, 2];

        let error = validate_textured_draw_buffers(&vertex_bytes, &index_bytes, 1).unwrap_err();

        assert_eq!(
            error,
            GpuBootstrapError::SmokeReadback("textured draw buffers must be non-empty".to_owned())
        );
    }

    #[test]
    fn background_draw_buffer_layout_reports_checked_sizes() {
        let vertex_bytes = [1_u8, 2, 3, 4];
        let index_bytes = [5_u8, 6, 7, 8];

        let layout = validate_background_draw_buffers(&vertex_bytes, &index_bytes, 1).unwrap();

        assert_eq!(
            layout,
            DrawBufferLayout {
                vertex_buffer_size: 4,
                index_buffer_size: 4,
                index_count: 1,
            }
        );
    }

    #[test]
    fn background_draw_buffer_layout_rejects_empty_buffers() {
        let vertex_bytes = [];
        let index_bytes = [1_u8, 2, 3, 4];

        let error = validate_background_draw_buffers(&vertex_bytes, &index_bytes, 1).unwrap_err();

        assert_eq!(
            error,
            GpuBootstrapError::SmokeReadback(
                "background draw buffers must be non-empty".to_owned()
            )
        );
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn textured_index_count_rejects_values_larger_than_wgpu_draw_range() {
        let error = checked_textured_index_count(usize::MAX).unwrap_err();

        assert_eq!(
            error,
            GpuBootstrapError::SmokeReadback("terminal text index count is too large".to_owned())
        );
    }
}
