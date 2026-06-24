use super::solid_quads::{
    BackgroundDrawInput, SolidQuadDrawLabels, prepare_solid_quad_draw,
    validate_background_draw_input,
};
use crate::native_gpu::draw_buffers::validate_textured_draw_buffers;
use crate::native_gpu::readback::read_texture_rgba8;
use crate::native_gpu::{GpuBootstrapError, UploadPattern, UploadPatternLayout};

mod pipeline;
mod textures;

use pipeline::{
    textured_bind_group, textured_bind_group_layout, textured_pipeline, textured_sampler,
};
use textures::{create_source_texture, create_target_texture};

pub(super) struct TexturedDrawInput<'a> {
    pub(super) pattern: &'a UploadPattern,
    pub(super) source_layout: UploadPatternLayout,
    pub(super) background: Option<BackgroundDrawInput>,
    pub(super) decoration: Option<BackgroundDrawInput>,
    pub(super) cursor: Option<BackgroundDrawInput>,
    pub(super) vertex_bytes: &'a [u8],
    pub(super) index_bytes: &'a [u8],
    pub(super) index_count: u32,
    pub(super) index_format: wgpu::IndexFormat,
    pub(super) width: u32,
    pub(super) height: u32,
}

pub(super) fn draw_textured_vertices_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    input: TexturedDrawInput<'_>,
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    let buffer_layout =
        validate_textured_draw_buffers(input.vertex_bytes, input.index_bytes, input.index_count)?;
    let background_layout = input
        .background
        .as_ref()
        .map(validate_background_draw_input)
        .transpose()?;
    let decoration_layout = input
        .decoration
        .as_ref()
        .map(validate_background_draw_input)
        .transpose()?;
    let cursor_layout = input
        .cursor
        .as_ref()
        .map(validate_background_draw_input)
        .transpose()?;

    let source = create_source_texture(device, queue, &input);
    let target = create_target_texture(device, input.width, input.height);
    let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = textured_sampler(device);
    let bind_group_layout = textured_bind_group_layout(device);
    let bind_group = textured_bind_group(device, &bind_group_layout, &source_view, &sampler);
    let pipeline = textured_pipeline(device, &bind_group_layout);
    let background_draw = prepare_solid_quad_draw(
        device,
        queue,
        input.background.as_ref(),
        background_layout,
        SolidQuadDrawLabels {
            shader: "gromaq-background-quad-shader",
            pipeline_layout: "gromaq-background-quad-pipeline-layout",
            pipeline: "gromaq-background-quad-pipeline",
            vertices: "gromaq-background-quad-vertices",
            indices: "gromaq-background-quad-indices",
        },
    );
    let decoration_draw = prepare_solid_quad_draw(
        device,
        queue,
        input.decoration.as_ref(),
        decoration_layout,
        SolidQuadDrawLabels {
            shader: "gromaq-decoration-quad-shader",
            pipeline_layout: "gromaq-decoration-quad-pipeline-layout",
            pipeline: "gromaq-decoration-quad-pipeline",
            vertices: "gromaq-decoration-quad-vertices",
            indices: "gromaq-decoration-quad-indices",
        },
    );
    let cursor_draw = prepare_solid_quad_draw(
        device,
        queue,
        input.cursor.as_ref(),
        cursor_layout,
        SolidQuadDrawLabels {
            shader: "gromaq-cursor-quad-shader",
            pipeline_layout: "gromaq-cursor-quad-pipeline-layout",
            pipeline: "gromaq-cursor-quad-pipeline",
            vertices: "gromaq-cursor-quad-vertices",
            indices: "gromaq-cursor-quad-indices",
        },
    );
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gromaq-textured-quad-vertices"),
        size: buffer_layout.vertex_buffer_size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gromaq-textured-quad-indices"),
        size: buffer_layout.index_buffer_size,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, input.vertex_bytes);
    queue.write_buffer(&index_buffer, 0, input.index_bytes);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gromaq-textured-quad-encoder"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gromaq-textured-quad-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if let Some(background_draw) = &background_draw {
            background_draw.draw(&mut pass);
        }
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), input.index_format);
        pass.draw_indexed(0..buffer_layout.index_count, 0, 0..1);
        if let Some(decoration_draw) = &decoration_draw {
            decoration_draw.draw(&mut pass);
        }
        if let Some(cursor_draw) = &cursor_draw {
            cursor_draw.draw(&mut pass);
        }
    }
    queue.submit([encoder.finish()]);
    read_texture_rgba8(device, queue, &target, input.width, input.height)
}
