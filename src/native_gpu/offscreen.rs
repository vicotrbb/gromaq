use std::borrow::Cow;

use super::draw_buffers::{
    DrawBufferLayout, checked_textured_index_count, validate_background_draw_buffers,
    validate_textured_draw_buffers,
};
use super::quad_bytes::{
    background_quad_index_bytes, background_quad_vertex_bytes, glyph_quad_index_bytes,
    glyph_quad_vertex_bytes, textured_quad_index_bytes, textured_quad_vertex_bytes,
};
use super::readback::read_texture_rgba8;
use super::shaders::{BACKGROUND_QUAD_WGSL, TEXTURED_QUAD_WGSL};
use super::{GpuBootstrapError, UploadPattern, UploadPatternLayout};
use crate::renderer::{BackgroundQuadBatch, GlyphAtlasImage, GlyphQuadBatch};

pub(super) fn clear_offscreen_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    color: [f64; 4],
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gromaq-smoke-target"),
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
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gromaq-smoke-encoder"),
    });
    {
        let attachment = wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: color[0],
                    g: color[1],
                    b: color[2],
                    a: color[3],
                }),
                store: wgpu::StoreOp::Store,
            },
        };
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gromaq-smoke-clear-pass"),
            color_attachments: &[Some(attachment)],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    queue.submit([encoder.finish()]);
    read_texture_rgba8(device, queue, &texture, width, height)
}

pub(super) fn upload_rgba8_and_readback(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pattern: &UploadPattern,
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    let layout = pattern.rgba8_layout()?;
    if pattern.rgba.len() != layout.expected_len {
        return Err(GpuBootstrapError::SmokeReadback(format!(
            "upload pattern has {} bytes, expected {}",
            pattern.rgba.len(),
            layout.expected_len
        )));
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gromaq-upload-smoke-texture"),
        size: wgpu::Extent3d {
            width: pattern.width,
            height: pattern.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        &pattern.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(layout.row_bytes),
            rows_per_image: Some(pattern.height),
        },
        wgpu::Extent3d {
            width: pattern.width,
            height: pattern.height,
            depth_or_array_layers: 1,
        },
    );
    read_texture_rgba8(device, queue, &texture, pattern.width, pattern.height)
}

fn validate_textured_source_pattern(
    pattern: &UploadPattern,
) -> std::result::Result<UploadPatternLayout, GpuBootstrapError> {
    let layout = pattern.rgba8_layout()?;
    if pattern.rgba.len() != layout.expected_len {
        return Err(GpuBootstrapError::SmokeReadback(format!(
            "textured quad source has {} bytes, expected {}",
            pattern.rgba.len(),
            layout.expected_len
        )));
    }
    Ok(layout)
}

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

struct TexturedDrawInput<'a> {
    pattern: &'a UploadPattern,
    source_layout: UploadPatternLayout,
    background: Option<BackgroundDrawInput>,
    decoration: Option<BackgroundDrawInput>,
    cursor: Option<BackgroundDrawInput>,
    vertex_bytes: &'a [u8],
    index_bytes: &'a [u8],
    index_count: u32,
    index_format: wgpu::IndexFormat,
    width: u32,
    height: u32,
}

struct BackgroundDrawInput {
    vertex_bytes: Vec<u8>,
    index_bytes: Vec<u8>,
    index_count: u32,
}

fn validate_background_draw_input(
    input: &BackgroundDrawInput,
) -> std::result::Result<DrawBufferLayout, GpuBootstrapError> {
    validate_background_draw_buffers(&input.vertex_bytes, &input.index_bytes, input.index_count)
}

fn draw_textured_vertices_rgba8(
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

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gromaq-textured-quad-target"),
        size: wgpu::Extent3d {
            width: input.width,
            height: input.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("gromaq-textured-quad-sampler"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("gromaq-textured-quad-bind-group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&source_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gromaq-textured-quad-shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(TEXTURED_QUAD_WGSL)),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("gromaq-textured-quad-pipeline-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("gromaq-textured-quad-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
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
    let background_draw = if let (Some(background), Some(layout)) =
        (&input.background, background_layout)
    {
        let background_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gromaq-background-quad-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(BACKGROUND_QUAD_WGSL)),
        });
        let background_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("gromaq-background-quad-pipeline-layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });
        let background_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gromaq-background-quad-pipeline"),
            layout: Some(&background_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &background_shader,
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
                module: &background_shader,
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
        let background_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gromaq-background-quad-vertices"),
            size: layout.vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let background_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gromaq-background-quad-indices"),
            size: layout.index_buffer_size,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&background_vertex_buffer, 0, &background.vertex_bytes);
        queue.write_buffer(&background_index_buffer, 0, &background.index_bytes);
        Some((
            background_pipeline,
            background_vertex_buffer,
            background_index_buffer,
            layout.index_count,
        ))
    } else {
        None
    };
    let decoration_draw = if let (Some(decoration), Some(layout)) =
        (&input.decoration, decoration_layout)
    {
        let decoration_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gromaq-decoration-quad-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(BACKGROUND_QUAD_WGSL)),
        });
        let decoration_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("gromaq-decoration-quad-pipeline-layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });
        let decoration_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gromaq-decoration-quad-pipeline"),
            layout: Some(&decoration_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &decoration_shader,
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
                module: &decoration_shader,
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
        let decoration_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gromaq-decoration-quad-vertices"),
            size: layout.vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let decoration_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gromaq-decoration-quad-indices"),
            size: layout.index_buffer_size,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&decoration_vertex_buffer, 0, &decoration.vertex_bytes);
        queue.write_buffer(&decoration_index_buffer, 0, &decoration.index_bytes);
        Some((
            decoration_pipeline,
            decoration_vertex_buffer,
            decoration_index_buffer,
            layout.index_count,
        ))
    } else {
        None
    };
    let cursor_draw = if let (Some(cursor), Some(layout)) = (&input.cursor, cursor_layout) {
        let cursor_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gromaq-cursor-quad-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(BACKGROUND_QUAD_WGSL)),
        });
        let cursor_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("gromaq-cursor-quad-pipeline-layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });
        let cursor_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gromaq-cursor-quad-pipeline"),
            layout: Some(&cursor_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &cursor_shader,
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
                module: &cursor_shader,
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
        let cursor_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gromaq-cursor-quad-vertices"),
            size: layout.vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cursor_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gromaq-cursor-quad-indices"),
            size: layout.index_buffer_size,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&cursor_vertex_buffer, 0, &cursor.vertex_bytes);
        queue.write_buffer(&cursor_index_buffer, 0, &cursor.index_bytes);
        Some((
            cursor_pipeline,
            cursor_vertex_buffer,
            cursor_index_buffer,
            layout.index_count,
        ))
    } else {
        None
    };
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
        if let Some((background_pipeline, vertex_buffer, index_buffer, index_count)) =
            &background_draw
        {
            pass.set_pipeline(background_pipeline);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..*index_count, 0, 0..1);
        }
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), input.index_format);
        pass.draw_indexed(0..buffer_layout.index_count, 0, 0..1);
        if let Some((decoration_pipeline, vertex_buffer, index_buffer, index_count)) =
            &decoration_draw
        {
            pass.set_pipeline(decoration_pipeline);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..*index_count, 0, 0..1);
        }
        if let Some((cursor_pipeline, vertex_buffer, index_buffer, index_count)) = &cursor_draw {
            pass.set_pipeline(cursor_pipeline);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..*index_count, 0, 0..1);
        }
    }
    queue.submit([encoder.finish()]);
    read_texture_rgba8(device, queue, &target, input.width, input.height)
}
