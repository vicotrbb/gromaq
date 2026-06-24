use std::borrow::Cow;

use super::solid_quads::{
    BackgroundDrawInput, SolidQuadDrawLabels, prepare_solid_quad_draw,
    validate_background_draw_input,
};
use crate::native_gpu::draw_buffers::validate_textured_draw_buffers;
use crate::native_gpu::readback::read_texture_rgba8;
use crate::native_gpu::shaders::TEXTURED_QUAD_WGSL;
use crate::native_gpu::{GpuBootstrapError, UploadPattern, UploadPatternLayout};

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
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("gromaq-textured-quad-sampler"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });
    let bind_group_layout = textured_bind_group_layout(device);
    let bind_group = textured_bind_group(device, &bind_group_layout, &source_view, &sampler);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gromaq-textured-quad-shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(TEXTURED_QUAD_WGSL)),
    });
    let pipeline = textured_pipeline(device, &bind_group_layout, &shader);
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

fn create_source_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    input: &TexturedDrawInput<'_>,
) -> wgpu::Texture {
    let source = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gromaq-textured-quad-source"),
        size: wgpu::Extent3d {
            width: input.pattern.width,
            height: input.pattern.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    queue.write_texture(
        source.as_image_copy(),
        &input.pattern.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(input.source_layout.row_bytes),
            rows_per_image: Some(input.pattern.height),
        },
        wgpu::Extent3d {
            width: input.pattern.width,
            height: input.pattern.height,
            depth_or_array_layers: 1,
        },
    );
    source
}

fn create_target_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gromaq-textured-quad-target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn textured_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("gromaq-textured-quad-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn textured_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("gromaq-textured-quad-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

fn textured_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("gromaq-textured-quad-pipeline-layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("gromaq-textured-quad-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 32,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 8,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 16,
                        shader_location: 2,
                    },
                ],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
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
    })
}
