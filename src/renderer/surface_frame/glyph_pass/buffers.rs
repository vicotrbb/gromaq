use crate::renderer::GlyphQuadBatch;

use super::super::super::surface_buffers::{
    surface_glyph_index_bytes, surface_glyph_vertex_bytes, validate_surface_glyph_buffers,
};
use super::super::SurfaceFrameError;

pub(super) struct SurfaceGlyphDrawBuffers {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) index_buffer: wgpu::Buffer,
    pub(super) index_count: u32,
}

pub(super) fn prepare_surface_glyph_draw_buffers(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    batch: &GlyphQuadBatch,
    target_width: u32,
    target_height: u32,
) -> std::result::Result<Option<SurfaceGlyphDrawBuffers>, SurfaceFrameError> {
    if batch.quads.is_empty() {
        return Ok(None);
    }

    let vertex_bytes = surface_glyph_vertex_bytes(batch, target_width, target_height)?;
    let index_bytes = surface_glyph_index_bytes(batch);
    let buffer_layout =
        validate_surface_glyph_buffers(&vertex_bytes, &index_bytes, batch.indices.len())?;
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gromaq-surface-glyph-vertices"),
        size: buffer_layout.vertex_buffer_size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gromaq-surface-glyph-indices"),
        size: buffer_layout.index_buffer_size,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, &vertex_bytes);
    queue.write_buffer(&index_buffer, 0, &index_bytes);

    Ok(Some(SurfaceGlyphDrawBuffers {
        vertex_buffer,
        index_buffer,
        index_count: buffer_layout.index_count,
    }))
}
