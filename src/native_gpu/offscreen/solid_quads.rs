use std::borrow::Cow;

use crate::native_gpu::GpuBootstrapError;
use crate::native_gpu::draw_buffers::{DrawBufferLayout, validate_background_draw_buffers};
use crate::native_gpu::shaders::BACKGROUND_QUAD_WGSL;

pub(super) struct BackgroundDrawInput {
    pub(super) vertex_bytes: Vec<u8>,
    pub(super) index_bytes: Vec<u8>,
    pub(super) index_count: u32,
}

pub(super) struct SolidQuadDraw {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl SolidQuadDraw {
    pub(super) fn draw<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SolidQuadDrawLabels {
    pub(super) shader: &'static str,
    pub(super) pipeline_layout: &'static str,
    pub(super) pipeline: &'static str,
    pub(super) vertices: &'static str,
    pub(super) indices: &'static str,
}

pub(super) fn validate_background_draw_input(
    input: &BackgroundDrawInput,
) -> std::result::Result<DrawBufferLayout, GpuBootstrapError> {
    validate_background_draw_buffers(&input.vertex_bytes, &input.index_bytes, input.index_count)
}

pub(super) fn prepare_solid_quad_draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    input: Option<&BackgroundDrawInput>,
    layout: Option<DrawBufferLayout>,
    labels: SolidQuadDrawLabels,
) -> Option<SolidQuadDraw> {
    let (Some(input), Some(layout)) = (input, layout) else {
        return None;
    };

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(labels.shader),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(BACKGROUND_QUAD_WGSL)),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(labels.pipeline_layout),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(labels.pipeline),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 24,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 8,
                        shader_location: 1,
                    },
                ],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(labels.vertices),
        size: layout.vertex_buffer_size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(labels.indices),
        size: layout.index_buffer_size,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, &input.vertex_bytes);
    queue.write_buffer(&index_buffer, 0, &input.index_bytes);

    Some(SolidQuadDraw {
        pipeline,
        vertex_buffer,
        index_buffer,
        index_count: layout.index_count,
    })
}
