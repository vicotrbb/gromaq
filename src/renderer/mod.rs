//! GPU renderer boundary.

use std::borrow::Cow;

use thiserror::Error;

use crate::config::GromaqConfig;
use crate::dirty::DirtyRegion;
use crate::error::Result;
use crate::grid::GridSnapshot;
use crate::terminal::CursorSnapshot;

mod atlas;
mod color;
mod plan;
mod prepared_frame;
mod quads;
mod scheduler;
mod surface;
mod surface_buffers;

pub use atlas::{
    GlyphAtlas, GlyphAtlasConfig, GlyphAtlasImage, GlyphAtlasMetrics, GlyphBitmap, GlyphEntry,
    GlyphImageError, GlyphKey, GlyphKeyText,
};
pub use plan::{
    PlannedBackground, PlannedGlyph, PlannedTextDecoration, RenderPlan, RenderPlanner,
    TextDecorationKind,
};
pub use prepared_frame::{PreparedSurfaceGlyphFrame, SurfaceGlyphFrame};
pub use quads::{
    BackgroundQuad, BackgroundQuadBatch, BackgroundQuadConfig, BackgroundQuadError,
    BackgroundQuadPlanner, BackgroundVertex, CursorQuadConfig, CursorQuadPlanner, GlyphQuad,
    GlyphQuadBatch, GlyphQuadConfig, GlyphQuadError, GlyphQuadPlanner, GlyphVertex,
    TextDecorationQuadConfig, TextDecorationQuadPlanner,
};
pub use scheduler::{FrameDecision, FrameScheduler, FrameSchedulerMetrics, RenderReason};
pub use surface::{
    SurfaceBackend, SurfaceConfigError, SurfaceConfigPlanner, SurfaceConfigurationController,
    SurfaceLifecycle, SurfaceLifecycleAction,
};
#[cfg(test)]
use surface_buffers::{
    SurfaceGlyphAtlasLayout, SurfaceGlyphBufferLayout, surface_background_vertex_byte_capacity,
    surface_glyph_vertex_byte_capacity,
};
use surface_buffers::{
    surface_background_index_bytes, surface_background_vertex_bytes, surface_glyph_index_bytes,
    surface_glyph_vertex_bytes, validate_surface_background_buffers,
    validate_surface_glyph_buffers, validate_surface_glyph_frame,
};

const DEFAULT_RENDERER_FONT_SIZE_PX: u16 = 14;
const DEFAULT_GLYPH_ATLAS_CAPACITY: usize = 4096;

/// Renderer configuration for the GPU backend.
#[derive(Debug, Clone, PartialEq)]
pub struct RendererConfig {
    /// Target frames per second.
    pub target_fps: u32,
    /// Whether dirty-region rendering is required.
    pub dirty_regions: bool,
    /// Font size in pixels used for glyph planning and cache keys.
    pub font_size_px: u16,
    /// Clear color in RGBA linear space.
    pub clear_color: [f64; 4],
}

/// Errors produced while acquiring or presenting a native surface frame.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SurfaceFrameError {
    /// Surface acquisition timed out; the frame should be skipped.
    #[error("surface frame acquisition timed out")]
    Timeout,
    /// Surface is currently occluded; the frame should be skipped.
    #[error("surface is occluded")]
    Occluded,
    /// Surface configuration is outdated and must be refreshed.
    #[error("surface configuration is outdated")]
    Outdated,
    /// Surface was lost and must be recreated.
    #[error("surface was lost")]
    Lost,
    /// Surface acquisition hit a validation error.
    #[error("surface frame acquisition validation error")]
    Validation,
    /// A terminal glyph frame could not be rendered.
    #[error("invalid surface frame: {0}")]
    InvalidFrame(String),
}

/// Surface endpoint that can render and present a frame.
pub trait SurfaceFrameBackend {
    /// Clear the current surface frame to `clear_color` and present it.
    fn clear_and_present(
        &mut self,
        clear_color: [f64; 4],
    ) -> std::result::Result<(), SurfaceFrameError>;

    /// Render terminal glyph quads into the current surface frame and present it.
    fn present_glyph_frame(
        &mut self,
        frame: SurfaceGlyphFrame<'_>,
    ) -> std::result::Result<(), SurfaceFrameError>;
}

/// Concrete `wgpu` surface backend used by the native app once a window surface exists.
pub struct WgpuSurfaceBackend<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    current_format: Option<wgpu::TextureFormat>,
}

impl<'a> WgpuSurfaceBackend<'a> {
    /// Create a surface backend from a `wgpu` surface and device.
    pub fn new(surface: wgpu::Surface<'a>, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self {
            surface,
            device: device.clone(),
            queue: queue.clone(),
            current_format: None,
        }
    }
}

impl SurfaceBackend for WgpuSurfaceBackend<'_> {
    fn configure(&mut self, config: &wgpu::SurfaceConfiguration) {
        self.surface.configure(&self.device, config);
        self.current_format = Some(config.format);
    }
}

impl SurfaceFrameBackend for WgpuSurfaceBackend<'_> {
    fn clear_and_present(
        &mut self,
        clear_color: [f64; 4],
    ) -> std::result::Result<(), SurfaceFrameError> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout => return Err(SurfaceFrameError::Timeout),
            wgpu::CurrentSurfaceTexture::Occluded => return Err(SurfaceFrameError::Occluded),
            wgpu::CurrentSurfaceTexture::Outdated => return Err(SurfaceFrameError::Outdated),
            wgpu::CurrentSurfaceTexture::Lost => return Err(SurfaceFrameError::Lost),
            wgpu::CurrentSurfaceTexture::Validation => return Err(SurfaceFrameError::Validation),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gromaq-surface-clear-encoder"),
            });
        {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gromaq-surface-clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color[0],
                            g: clear_color[1],
                            b: clear_color[2],
                            a: clear_color[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }
        self.queue.submit([encoder.finish()]);
        frame.present();
        Ok(())
    }

    fn present_glyph_frame(
        &mut self,
        glyph_frame: SurfaceGlyphFrame<'_>,
    ) -> std::result::Result<(), SurfaceFrameError> {
        let Some(format) = self.current_format else {
            return Err(SurfaceFrameError::InvalidFrame(
                "surface must be configured before drawing terminal glyphs".to_owned(),
            ));
        };
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout => return Err(SurfaceFrameError::Timeout),
            wgpu::CurrentSurfaceTexture::Occluded => return Err(SurfaceFrameError::Occluded),
            wgpu::CurrentSurfaceTexture::Outdated => return Err(SurfaceFrameError::Outdated),
            wgpu::CurrentSurfaceTexture::Lost => return Err(SurfaceFrameError::Lost),
            wgpu::CurrentSurfaceTexture::Validation => return Err(SurfaceFrameError::Validation),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        render_glyph_frame_to_view(&self.device, &self.queue, &view, format, glyph_frame)?;
        frame.present();
        Ok(())
    }
}

fn render_glyph_frame_to_view(
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

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            target_fps: 144,
            dirty_regions: true,
            font_size_px: DEFAULT_RENDERER_FONT_SIZE_PX,
            clear_color: [0.02, 0.02, 0.025, 1.0],
        }
    }
}

impl RendererConfig {
    /// Build renderer configuration from validated user configuration.
    pub fn from_gromaq_config(config: &GromaqConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            target_fps: config.performance.target_fps,
            dirty_regions: config.performance.dirty_region_rendering,
            font_size_px: config.font.renderer_font_size_px(),
            ..Self::default()
        })
    }
}

/// Narrow GPU rendering interface.
pub trait GpuRenderer {
    /// Queue a terminal snapshot for rendering.
    fn render_frame(
        &mut self,
        grid: &GridSnapshot,
        cursor: CursorSnapshot,
        dirty_regions: &[DirtyRegion],
    ) -> Result<()>;
}

/// `wgpu` backend marker and configuration holder.
#[derive(Debug)]
pub struct WgpuRenderer {
    config: RendererConfig,
    planner: RenderPlanner,
    glyph_atlas: GlyphAtlas,
    last_plan: Option<RenderPlan>,
}

impl WgpuRenderer {
    /// Create a renderer boundary. Device creation is part of the native UI bootstrap.
    pub fn new(config: RendererConfig) -> Result<Self> {
        let atlas_config = GlyphAtlasConfig::new(DEFAULT_GLYPH_ATLAS_CAPACITY)?;
        Ok(Self {
            planner: RenderPlanner::new(config.font_size_px),
            config,
            glyph_atlas: GlyphAtlas::new(atlas_config),
            last_plan: None,
        })
    }

    /// Access renderer configuration.
    pub fn config(&self) -> &RendererConfig {
        &self.config
    }

    /// Replace renderer configuration for future frame planning.
    pub fn reconfigure(&mut self, config: RendererConfig) {
        self.planner = RenderPlanner::new(config.font_size_px);
        self.config = config;
        self.last_plan = None;
    }

    /// Return the most recent planned frame.
    pub fn last_plan(&self) -> Option<&RenderPlan> {
        self.last_plan.as_ref()
    }

    /// Return internal glyph atlas metrics.
    pub fn glyph_atlas_metrics(&self) -> GlyphAtlasMetrics {
        self.glyph_atlas.metrics()
    }
}

impl GpuRenderer for WgpuRenderer {
    fn render_frame(
        &mut self,
        grid: &GridSnapshot,
        cursor: CursorSnapshot,
        dirty_regions: &[DirtyRegion],
    ) -> Result<()> {
        let full_viewport;
        let regions = if self.config.dirty_regions {
            dirty_regions
        } else {
            full_viewport = [DirtyRegion {
                row: 0,
                col: 0,
                rows: grid.rows,
                cols: grid.cols,
            }];
            &full_viewport
        };
        let plan = self
            .planner
            .plan_frame(grid, cursor, regions, &mut self.glyph_atlas)?;
        self.last_plan = Some(plan);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn one_quad_batch() -> GlyphQuadBatch {
        let quad = GlyphQuad {
            text: "A".to_owned(),
            ch: 'A',
            atlas_entry: GlyphEntry {
                slot: 0,
                generation: 0,
            },
            vertices: [
                GlyphVertex {
                    position: [0.0, 0.0],
                    uv: [0.0, 0.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                GlyphVertex {
                    position: [1.0, 0.0],
                    uv: [1.0, 0.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                GlyphVertex {
                    position: [1.0, 1.0],
                    uv: [1.0, 1.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                GlyphVertex {
                    position: [0.0, 1.0],
                    uv: [0.0, 1.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
            ],
        };
        GlyphQuadBatch {
            quads: vec![quad],
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    }

    #[test]
    fn frame_scheduler_dropped_frame_metrics_saturate() {
        let mut scheduler = FrameScheduler::new(1).unwrap();
        scheduler.metrics_mut().dropped_frames = u64::MAX - 1;
        let start = Instant::now();
        scheduler.record_presented(start);

        scheduler.record_presented(start + Duration::from_secs(4));

        assert_eq!(scheduler.metrics().dropped_frames, u64::MAX);
    }

    #[test]
    fn frame_scheduler_presented_frame_metrics_saturate() {
        let mut scheduler = FrameScheduler::new(144).unwrap();
        scheduler.metrics_mut().frames_presented = u64::MAX;

        scheduler.record_presented(Instant::now());

        assert_eq!(scheduler.metrics().frames_presented, u64::MAX);
    }

    #[test]
    fn surface_glyph_frame_validation_computes_checked_atlas_layout() {
        let atlas = GlyphAtlasImage {
            width: 2,
            height: 2,
            rgba: vec![255; 16],
            occupied_slots: 1,
        };
        let batch = one_quad_batch();

        let layout = validate_surface_glyph_frame(SurfaceGlyphFrame {
            atlas: &atlas,
            background_batch: &BackgroundQuadBatch::default(),
            batch: &batch,
            decoration_batch: &BackgroundQuadBatch::default(),
            cursor_batch: &BackgroundQuadBatch::default(),
            width: 16,
            height: 16,
            clear_color: [0.0, 0.0, 0.0, 1.0],
        })
        .unwrap();

        assert_eq!(
            layout,
            SurfaceGlyphAtlasLayout {
                row_bytes: 8,
                expected_len: 16,
            }
        );
    }

    #[test]
    fn surface_glyph_frame_validation_rejects_overflowing_atlas_row_size() {
        let atlas = GlyphAtlasImage {
            width: u32::MAX,
            height: 1,
            rgba: Vec::new(),
            occupied_slots: 0,
        };
        let batch = one_quad_batch();

        let error = validate_surface_glyph_frame(SurfaceGlyphFrame {
            atlas: &atlas,
            background_batch: &BackgroundQuadBatch::default(),
            batch: &batch,
            decoration_batch: &BackgroundQuadBatch::default(),
            cursor_batch: &BackgroundQuadBatch::default(),
            width: 16,
            height: 16,
            clear_color: [0.0, 0.0, 0.0, 1.0],
        })
        .unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame("surface glyph atlas row size is too large".to_owned())
        );
    }

    #[test]
    fn surface_glyph_frame_validation_accepts_background_only_batches() {
        let atlas = GlyphAtlasImage {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
            occupied_slots: 0,
        };
        let background_batch = BackgroundQuadBatch {
            quads: vec![BackgroundQuad {
                row: 0,
                col: 0,
                cols: 1,
                vertices: [
                    BackgroundVertex {
                        position: [0.0, 0.0],
                        color_rgba: [1.0, 0.0, 0.0, 1.0],
                    },
                    BackgroundVertex {
                        position: [1.0, 0.0],
                        color_rgba: [1.0, 0.0, 0.0, 1.0],
                    },
                    BackgroundVertex {
                        position: [1.0, 1.0],
                        color_rgba: [1.0, 0.0, 0.0, 1.0],
                    },
                    BackgroundVertex {
                        position: [0.0, 1.0],
                        color_rgba: [1.0, 0.0, 0.0, 1.0],
                    },
                ],
            }],
            indices: vec![0, 1, 2, 0, 2, 3],
        };

        let layout = validate_surface_glyph_frame(SurfaceGlyphFrame {
            atlas: &atlas,
            background_batch: &background_batch,
            batch: &GlyphQuadBatch::default(),
            decoration_batch: &BackgroundQuadBatch::default(),
            cursor_batch: &BackgroundQuadBatch::default(),
            width: 1,
            height: 1,
            clear_color: [0.0, 0.0, 0.0, 1.0],
        })
        .unwrap();

        assert_eq!(
            layout,
            SurfaceGlyphAtlasLayout {
                row_bytes: 4,
                expected_len: 4,
            }
        );
    }

    #[test]
    fn surface_glyph_frame_validation_accepts_cursor_only_batches() {
        let atlas = GlyphAtlasImage {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
            occupied_slots: 0,
        };
        let cursor_batch = BackgroundQuadBatch {
            quads: vec![BackgroundQuad {
                row: 0,
                col: 0,
                cols: 1,
                vertices: [
                    BackgroundVertex {
                        position: [0.0, 0.0],
                        color_rgba: [1.0, 1.0, 1.0, 1.0],
                    },
                    BackgroundVertex {
                        position: [1.0, 0.0],
                        color_rgba: [1.0, 1.0, 1.0, 1.0],
                    },
                    BackgroundVertex {
                        position: [1.0, 1.0],
                        color_rgba: [1.0, 1.0, 1.0, 1.0],
                    },
                    BackgroundVertex {
                        position: [0.0, 1.0],
                        color_rgba: [1.0, 1.0, 1.0, 1.0],
                    },
                ],
            }],
            indices: vec![0, 1, 2, 0, 2, 3],
        };

        let layout = validate_surface_glyph_frame(SurfaceGlyphFrame {
            atlas: &atlas,
            background_batch: &BackgroundQuadBatch::default(),
            batch: &GlyphQuadBatch::default(),
            decoration_batch: &BackgroundQuadBatch::default(),
            cursor_batch: &cursor_batch,
            width: 1,
            height: 1,
            clear_color: [0.0, 0.0, 0.0, 1.0],
        })
        .unwrap();

        assert_eq!(
            layout,
            SurfaceGlyphAtlasLayout {
                row_bytes: 4,
                expected_len: 4,
            }
        );
    }

    #[test]
    fn surface_glyph_buffer_validation_reports_checked_sizes() {
        let vertex_bytes = [1_u8, 2, 3, 4];
        let index_bytes = [5_u8, 6, 7, 8];

        let layout = validate_surface_glyph_buffers(&vertex_bytes, &index_bytes, 1).unwrap();

        assert_eq!(
            layout,
            SurfaceGlyphBufferLayout {
                vertex_buffer_size: 4,
                index_buffer_size: 4,
                index_count: 1,
            }
        );
    }

    #[test]
    fn surface_background_buffer_validation_reports_checked_sizes() {
        let vertex_bytes = [1_u8, 2, 3, 4];
        let index_bytes = [5_u8, 6, 7, 8];

        let layout = validate_surface_background_buffers(&vertex_bytes, &index_bytes, 1).unwrap();

        assert_eq!(
            layout,
            SurfaceGlyphBufferLayout {
                vertex_buffer_size: 4,
                index_buffer_size: 4,
                index_count: 1,
            }
        );
    }

    #[test]
    fn surface_glyph_vertex_byte_capacity_uses_checked_multiplication() {
        assert_eq!(surface_glyph_vertex_byte_capacity(2).unwrap(), 256);

        let error = surface_glyph_vertex_byte_capacity((usize::MAX / 128) + 1).unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame("surface glyph vertex bytes are too large".to_owned())
        );
    }

    #[test]
    fn surface_background_vertex_byte_capacity_uses_checked_multiplication() {
        assert_eq!(surface_background_vertex_byte_capacity(2).unwrap(), 192);

        let error = surface_background_vertex_byte_capacity((usize::MAX / 96) + 1).unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame(
                "surface background vertex bytes are too large".to_owned()
            )
        );
    }

    #[test]
    fn surface_glyph_buffer_validation_rejects_empty_buffers() {
        let vertex_bytes = [];
        let index_bytes = [1_u8, 2, 3, 4];

        let error = validate_surface_glyph_buffers(&vertex_bytes, &index_bytes, 1).unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame(
                "surface glyph draw buffers must be non-empty".to_owned()
            )
        );
    }

    #[test]
    fn surface_background_buffer_validation_rejects_empty_buffers() {
        let vertex_bytes = [];
        let index_bytes = [1_u8, 2, 3, 4];

        let error =
            validate_surface_background_buffers(&vertex_bytes, &index_bytes, 1).unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame(
                "surface background draw buffers must be non-empty".to_owned()
            )
        );
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn surface_glyph_buffer_validation_rejects_oversized_index_count() {
        let vertex_bytes = [1_u8, 2, 3, 4];
        let index_bytes = [5_u8, 6, 7, 8];

        let error =
            validate_surface_glyph_buffers(&vertex_bytes, &index_bytes, usize::MAX).unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame("surface glyph index count is too large".to_owned())
        );
    }
}
