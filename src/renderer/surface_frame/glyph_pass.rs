use std::borrow::Cow;

use super::super::SurfaceGlyphFrame;
use super::super::surface_buffers::{
    surface_background_index_bytes, surface_background_vertex_bytes, surface_glyph_index_bytes,
    surface_glyph_vertex_bytes, validate_surface_background_buffers,
    validate_surface_glyph_buffers, validate_surface_glyph_frame,
};
use super::SurfaceFrameError;

pub(super) fn render_glyph_frame_to_view(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    format: wgpu::TextureFormat,
    frame: SurfaceGlyphFrame<'_>,
) -> std::result::Result<(), SurfaceFrameError> {
    let atlas_layout = validate_surface_glyph_frame(frame)?;
    let atlas = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gromaq-surface-glyph-atlas"),
        size: wgpu::Extent3d {
            width: frame.atlas.width,
            height: frame.atlas.height,
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
        atlas.as_image_copy(),
        &frame.atlas.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(atlas_layout.row_bytes),
            rows_per_image: Some(frame.atlas.height),
        },
        wgpu::Extent3d {
            width: frame.atlas.width,
            height: frame.atlas.height,
            depth_or_array_layers: 1,
        },
    );
    let atlas_view = atlas.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("gromaq-surface-glyph-sampler"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("gromaq-surface-glyph-bind-group-layout"),
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
        label: Some("gromaq-surface-glyph-bind-group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&atlas_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gromaq-surface-glyph-shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SURFACE_GLYPH_WGSL)),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("gromaq-surface-glyph-pipeline-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("gromaq-surface-glyph-pipeline"),
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
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
    let background_draw = if frame.background_batch.quads.is_empty() {
        None
    } else {
        let background_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gromaq-surface-background-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SURFACE_BACKGROUND_WGSL)),
        });
        let background_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("gromaq-surface-background-pipeline-layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });
        let background_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gromaq-surface-background-pipeline"),
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
        let background_vertex_bytes =
            surface_background_vertex_bytes(frame.background_batch, frame.width, frame.height)?;
        let background_index_bytes = surface_background_index_bytes(frame.background_batch);
        let background_layout = validate_surface_background_buffers(
            &background_vertex_bytes,
            &background_index_bytes,
            frame.background_batch.indices.len(),
        )?;
        let background_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gromaq-surface-background-vertices"),
            size: background_layout.vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let background_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gromaq-surface-background-indices"),
            size: background_layout.index_buffer_size,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&background_vertex_buffer, 0, &background_vertex_bytes);
        queue.write_buffer(&background_index_buffer, 0, &background_index_bytes);
        Some((
            background_pipeline,
            background_vertex_buffer,
            background_index_buffer,
            background_layout.index_count,
        ))
    };
    let glyph_draw = if frame.batch.quads.is_empty() {
        None
    } else {
        let vertex_bytes = surface_glyph_vertex_bytes(frame.batch, frame.width, frame.height)?;
        let index_bytes = surface_glyph_index_bytes(frame.batch);
        let buffer_layout =
            validate_surface_glyph_buffers(&vertex_bytes, &index_bytes, frame.batch.indices.len())?;
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
        Some((vertex_buffer, index_buffer, buffer_layout.index_count))
    };
    let decoration_draw = if frame.decoration_batch.quads.is_empty() {
        None
    } else {
        let decoration_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gromaq-surface-decoration-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SURFACE_BACKGROUND_WGSL)),
        });
        let decoration_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("gromaq-surface-decoration-pipeline-layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });
        let decoration_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gromaq-surface-decoration-pipeline"),
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
        let decoration_vertex_bytes =
            surface_background_vertex_bytes(frame.decoration_batch, frame.width, frame.height)?;
        let decoration_index_bytes = surface_background_index_bytes(frame.decoration_batch);
        let decoration_layout = validate_surface_background_buffers(
            &decoration_vertex_bytes,
            &decoration_index_bytes,
            frame.decoration_batch.indices.len(),
        )?;
        let decoration_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gromaq-surface-decoration-vertices"),
            size: decoration_layout.vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let decoration_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gromaq-surface-decoration-indices"),
            size: decoration_layout.index_buffer_size,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&decoration_vertex_buffer, 0, &decoration_vertex_bytes);
        queue.write_buffer(&decoration_index_buffer, 0, &decoration_index_bytes);
        Some((
            decoration_pipeline,
            decoration_vertex_buffer,
            decoration_index_buffer,
            decoration_layout.index_count,
        ))
    };
    let cursor_draw = if frame.cursor_batch.quads.is_empty() {
        None
    } else {
        let cursor_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gromaq-surface-cursor-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SURFACE_BACKGROUND_WGSL)),
        });
        let cursor_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("gromaq-surface-cursor-pipeline-layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });
        let cursor_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gromaq-surface-cursor-pipeline"),
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
        let cursor_vertex_bytes =
            surface_background_vertex_bytes(frame.cursor_batch, frame.width, frame.height)?;
        let cursor_index_bytes = surface_background_index_bytes(frame.cursor_batch);
        let cursor_layout = validate_surface_background_buffers(
            &cursor_vertex_bytes,
            &cursor_index_bytes,
            frame.cursor_batch.indices.len(),
        )?;
        let cursor_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gromaq-surface-cursor-vertices"),
            size: cursor_layout.vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cursor_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gromaq-surface-cursor-indices"),
            size: cursor_layout.index_buffer_size,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&cursor_vertex_buffer, 0, &cursor_vertex_bytes);
        queue.write_buffer(&cursor_index_buffer, 0, &cursor_index_bytes);
        Some((
            cursor_pipeline,
            cursor_vertex_buffer,
            cursor_index_buffer,
            cursor_layout.index_count,
        ))
    };
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gromaq-surface-glyph-encoder"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gromaq-surface-glyph-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: frame.clear_color[0],
                        g: frame.clear_color[1],
                        b: frame.clear_color[2],
                        a: frame.clear_color[3],
                    }),
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
        if let Some((vertex_buffer, index_buffer, index_count)) = &glyph_draw {
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..*index_count, 0, 0..1);
        }
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
    Ok(())
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

const SURFACE_GLYPH_WGSL: &str = r#"
struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) foreground: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) foreground: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var output: VertexOut;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.foreground = input.foreground;
    return output;
}

@group(0) @binding(0) var atlas_texture: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let sample = textureSample(atlas_texture, atlas_sampler, input.uv);
    var rgb = sample.rgb;
    if (abs(sample.r - sample.g) + abs(sample.g - sample.b) < 0.03) {
        rgb = sample.rgb * input.foreground.rgb;
    }
    return vec4<f32>(rgb, sample.a * input.foreground.a);
}
"#;
