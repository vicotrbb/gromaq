use std::borrow::Cow;

use super::super::BackgroundQuadBatch;
use super::super::surface_buffers::{
    surface_background_index_bytes, surface_background_vertex_bytes,
    validate_surface_background_buffers,
};
use super::SurfaceFrameError;

pub(super) struct SolidDrawLabels {
    pub(super) shader: &'static str,
    pub(super) pipeline_layout: &'static str,
    pub(super) pipeline: &'static str,
    pub(super) vertex_buffer: &'static str,
    pub(super) index_buffer: &'static str,
}

pub(super) struct PreparedSolidDraw {
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) index_buffer: wgpu::Buffer,
    pub(super) index_count: u32,
}

pub(super) fn prepare_solid_draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    batch: &BackgroundQuadBatch,
    frame_width: u32,
    frame_height: u32,
    labels: SolidDrawLabels,
) -> std::result::Result<Option<PreparedSolidDraw>, SurfaceFrameError> {
    if batch.quads.is_empty() {
        return Ok(None);
    }

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(labels.shader),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SURFACE_BACKGROUND_WGSL)),
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
                format,
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
    let vertex_bytes = surface_background_vertex_bytes(batch, frame_width, frame_height)?;
    let index_bytes = surface_background_index_bytes(batch);
    let layout =
        validate_surface_background_buffers(&vertex_bytes, &index_bytes, batch.indices.len())?;
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(labels.vertex_buffer),
        size: layout.vertex_buffer_size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(labels.index_buffer),
        size: layout.index_buffer_size,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, &vertex_bytes);
    queue.write_buffer(&index_buffer, 0, &index_bytes);
    Ok(Some(PreparedSolidDraw {
        pipeline,
        vertex_buffer,
        index_buffer,
        index_count: layout.index_count,
    }))
}

const SURFACE_BACKGROUND_WGSL: &str = r#"
struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var output: VertexOut;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return input.color;
}
"#;
