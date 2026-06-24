use super::draw_buffers::checked_textured_index_count;
use super::quad_bytes::{
    background_quad_index_bytes, background_quad_vertex_bytes, glyph_quad_index_bytes,
    glyph_quad_vertex_bytes, textured_quad_index_bytes, textured_quad_vertex_bytes,
};
use super::{GpuBootstrapError, UploadPattern};
use crate::renderer::{BackgroundQuadBatch, GlyphAtlasImage, GlyphQuadBatch};
use solid_quads::BackgroundDrawInput;
use texture_io::validate_textured_source_pattern;
use textured_draw::{TexturedDrawInput, draw_textured_vertices_rgba8};

mod solid_quads;
mod texture_io;
mod textured_draw;

pub(super) use texture_io::{clear_offscreen_rgba8, upload_rgba8_and_readback};

pub(super) fn draw_textured_quad_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pattern: &UploadPattern,
    width: u32,
    height: u32,
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    if width == 0 || height == 0 {
        return Err(GpuBootstrapError::SmokeReadback(
            "textured quad dimensions must be non-zero".to_owned(),
        ));
    }
    let source_layout = validate_textured_source_pattern(pattern)?;

    draw_textured_vertices_rgba8(
        device,
        queue,
        TexturedDrawInput {
            pattern,
            source_layout,
            background: None,
            decoration: None,
            cursor: None,
            vertex_bytes: &textured_quad_vertex_bytes(),
            index_bytes: &textured_quad_index_bytes(),
            index_count: 6,
            index_format: wgpu::IndexFormat::Uint16,
            width,
            height,
        },
    )
}

pub(super) fn draw_glyph_quads_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    input: GlyphDrawInput<'_>,
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    if input.batch.quads.is_empty() {
        return Err(GpuBootstrapError::SmokeReadback(
            "terminal text draw batch is empty".to_owned(),
        ));
    }
    let pattern = UploadPattern::from_glyph_atlas_image(input.image);
    let source_layout = validate_textured_source_pattern(&pattern)?;
    let vertices = glyph_quad_vertex_bytes(input.batch, input.width, input.height)?;
    let indices = glyph_quad_index_bytes(input.batch);
    let index_count = checked_textured_index_count(input.batch.indices.len())?;
    let background = if input.background_batch.quads.is_empty() {
        None
    } else {
        Some(BackgroundDrawInput {
            vertex_bytes: background_quad_vertex_bytes(
                input.background_batch,
                input.width,
                input.height,
            )?,
            index_bytes: background_quad_index_bytes(input.background_batch),
            index_count: checked_textured_index_count(input.background_batch.indices.len())?,
        })
    };
    let decoration = if input.decoration_batch.quads.is_empty() {
        None
    } else {
        Some(BackgroundDrawInput {
            vertex_bytes: background_quad_vertex_bytes(
                input.decoration_batch,
                input.width,
                input.height,
            )?,
            index_bytes: background_quad_index_bytes(input.decoration_batch),
            index_count: checked_textured_index_count(input.decoration_batch.indices.len())?,
        })
    };
    let cursor = if input.cursor_batch.quads.is_empty() {
        None
    } else {
        Some(BackgroundDrawInput {
            vertex_bytes: background_quad_vertex_bytes(
                input.cursor_batch,
                input.width,
                input.height,
            )?,
            index_bytes: background_quad_index_bytes(input.cursor_batch),
            index_count: checked_textured_index_count(input.cursor_batch.indices.len())?,
        })
    };
    draw_textured_vertices_rgba8(
        device,
        queue,
        TexturedDrawInput {
            pattern: &pattern,
            source_layout,
            background,
            decoration,
            cursor,
            vertex_bytes: &vertices,
            index_bytes: &indices,
            index_count,
            index_format: wgpu::IndexFormat::Uint32,
            width: input.width,
            height: input.height,
        },
    )
}

pub(super) struct GlyphDrawInput<'a> {
    pub(super) image: &'a GlyphAtlasImage,
    pub(super) background_batch: &'a BackgroundQuadBatch,
    pub(super) batch: &'a GlyphQuadBatch,
    pub(super) decoration_batch: &'a BackgroundQuadBatch,
    pub(super) cursor_batch: &'a BackgroundQuadBatch,
    pub(super) width: u32,
    pub(super) height: u32,
}
