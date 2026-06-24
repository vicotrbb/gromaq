//! GPU renderer boundary.

use std::borrow::Cow;

use thiserror::Error;

use crate::cell::{Color, Style, UnderlineStyle};
use crate::config::GromaqConfig;
use crate::dirty::DirtyRegion;
use crate::error::Result;
use crate::grid::GridSnapshot;
use crate::terminal::CursorSnapshot;

mod atlas;
mod color;
mod quads;
mod scheduler;
mod surface;

pub use atlas::{
    GlyphAtlas, GlyphAtlasConfig, GlyphAtlasImage, GlyphAtlasMetrics, GlyphBitmap, GlyphEntry,
    GlyphImageError, GlyphKey, GlyphKeyText,
};
use color::{decoration_color_rgba8, style_background_rgba8};
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

/// Glyph frame data ready for presentation to a native surface.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceGlyphFrame<'a> {
    /// Packed glyph atlas image sampled by the frame.
    pub atlas: &'a GlyphAtlasImage,
    /// Solid background quads drawn before textured glyphs.
    pub background_batch: &'a BackgroundQuadBatch,
    /// Textured glyph quads and indices to draw.
    pub batch: &'a GlyphQuadBatch,
    /// Solid text-decoration quads drawn after textured glyphs.
    pub decoration_batch: &'a BackgroundQuadBatch,
    /// Solid cursor quads drawn after textured glyphs.
    pub cursor_batch: &'a BackgroundQuadBatch,
    /// Surface frame width in pixels.
    pub width: u32,
    /// Surface frame height in pixels.
    pub height: u32,
    /// Clear color used before drawing glyphs.
    pub clear_color: [f64; 4],
}

/// Owned terminal glyph frame prepared for presentation to a native surface.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedSurfaceGlyphFrame {
    atlas: GlyphAtlasImage,
    background_batch: BackgroundQuadBatch,
    batch: GlyphQuadBatch,
    decoration_batch: BackgroundQuadBatch,
    cursor_batch: BackgroundQuadBatch,
    width: u32,
    height: u32,
    clear_color: [f64; 4],
}

impl PreparedSurfaceGlyphFrame {
    /// Build an owned presentable glyph frame from a render plan and rasterized glyph bitmaps.
    pub fn from_render_plan(
        plan: &RenderPlan,
        glyphs: &[GlyphBitmap],
        clear_color: [f64; 4],
    ) -> std::result::Result<Self, SurfaceFrameError> {
        if plan.glyphs.is_empty() {
            return Err(SurfaceFrameError::InvalidFrame(
                "render plan contains no glyphs to present".to_owned(),
            ));
        }
        if glyphs.is_empty() {
            return Err(SurfaceFrameError::InvalidFrame(
                "surface glyph frame requires rasterized glyph bitmaps".to_owned(),
            ));
        }
        for planned in &plan.glyphs {
            if !glyphs
                .iter()
                .any(|glyph| glyph.entry == planned.atlas_entry)
            {
                return Err(SurfaceFrameError::InvalidFrame(format!(
                    "missing rasterized bitmap for atlas slot {} generation {}",
                    planned.atlas_entry.slot, planned.atlas_entry.generation
                )));
            }
        }

        let slot_width = glyphs.iter().map(|glyph| glyph.width).max().unwrap_or(0);
        let slot_height = glyphs.iter().map(|glyph| glyph.height).max().unwrap_or(0);
        if slot_width == 0 || slot_height == 0 {
            return Err(SurfaceFrameError::InvalidFrame(
                "rasterized glyph dimensions must be non-zero".to_owned(),
            ));
        }
        let width = checked_surface_frame_pixel_dimension(
            "surface glyph frame width",
            plan.viewport_cols,
            slot_width,
        )?;
        let height = checked_surface_frame_pixel_dimension(
            "surface glyph frame height",
            plan.viewport_rows,
            slot_height,
        )?;
        let padded = glyphs
            .iter()
            .map(|glyph| {
                glyph
                    .padded_to(slot_width, slot_height)
                    .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let columns = atlas_columns_for_glyphs(&padded);
        let atlas = GlyphAtlasImage::pack_rgba8(slot_width, slot_height, columns, &padded)
            .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?;
        let batch = GlyphQuadPlanner::new(GlyphQuadConfig {
            cell_width_px: slot_width,
            cell_height_px: slot_height,
            atlas_slot_width_px: slot_width,
            atlas_slot_height_px: slot_height,
            atlas_columns: columns,
            atlas_width_px: atlas.width,
            atlas_height_px: atlas.height,
        })
        .plan(plan)
        .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?;
        let background_batch = BackgroundQuadPlanner::new(BackgroundQuadConfig {
            cell_width_px: slot_width,
            cell_height_px: slot_height,
        })
        .plan(plan)
        .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?;
        let decoration_batch = TextDecorationQuadPlanner::new(TextDecorationQuadConfig {
            cell_width_px: slot_width,
            cell_height_px: slot_height,
        })
        .plan(plan)
        .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?;
        let cursor_batch = CursorQuadPlanner::new(CursorQuadConfig {
            cell_width_px: slot_width,
            cell_height_px: slot_height,
            color_rgba8: [229, 229, 229, 255],
        })
        .plan(plan)
        .map_err(|error| SurfaceFrameError::InvalidFrame(error.to_string()))?;
        Ok(Self {
            atlas,
            background_batch,
            batch,
            decoration_batch,
            cursor_batch,
            width,
            height,
            clear_color,
        })
    }

    /// Borrow this owned frame as a surface presentation frame.
    pub fn as_surface_glyph_frame(&self) -> SurfaceGlyphFrame<'_> {
        SurfaceGlyphFrame {
            atlas: &self.atlas,
            background_batch: &self.background_batch,
            batch: &self.batch,
            decoration_batch: &self.decoration_batch,
            cursor_batch: &self.cursor_batch,
            width: self.width,
            height: self.height,
            clear_color: self.clear_color,
        }
    }

    /// Packed atlas image for this frame.
    pub fn atlas(&self) -> &GlyphAtlasImage {
        &self.atlas
    }

    /// Glyph quad batch for this frame.
    pub fn batch(&self) -> &GlyphQuadBatch {
        &self.batch
    }

    /// Solid background quad batch for this frame.
    pub fn background_batch(&self) -> &BackgroundQuadBatch {
        &self.background_batch
    }

    /// Solid text-decoration quad batch for this frame.
    pub fn decoration_batch(&self) -> &BackgroundQuadBatch {
        &self.decoration_batch
    }

    /// Solid cursor quad batch for this frame.
    pub fn cursor_batch(&self) -> &BackgroundQuadBatch {
        &self.cursor_batch
    }
}

fn checked_surface_frame_pixel_dimension(
    label: &'static str,
    cells: u16,
    cell_size_px: u32,
) -> std::result::Result<u32, SurfaceFrameError> {
    u32::from(cells).checked_mul(cell_size_px).ok_or_else(|| {
        SurfaceFrameError::InvalidFrame(format!("{label} is too large to represent"))
    })
}

fn atlas_columns_for_glyphs(glyphs: &[GlyphBitmap]) -> u32 {
    let slots = glyphs
        .iter()
        .map(|glyph| u64::from(glyph.entry.slot))
        .max()
        .unwrap_or(0)
        + 1;
    let mut columns = 1_u32;
    while u64::from(columns) * u64::from(columns) < slots {
        columns += 1;
    }
    columns
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SurfaceGlyphAtlasLayout {
    row_bytes: u32,
    expected_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SurfaceGlyphBufferLayout {
    vertex_buffer_size: u64,
    index_buffer_size: u64,
    index_count: u32,
}

fn validate_surface_glyph_frame(
    frame: SurfaceGlyphFrame<'_>,
) -> std::result::Result<SurfaceGlyphAtlasLayout, SurfaceFrameError> {
    if frame.width == 0 || frame.height == 0 {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph frame dimensions must be non-zero".to_owned(),
        ));
    }
    if frame.atlas.width == 0 || frame.atlas.height == 0 {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph atlas dimensions must be non-zero".to_owned(),
        ));
    }
    let row_bytes = frame.atlas.width.checked_mul(4).ok_or_else(|| {
        SurfaceFrameError::InvalidFrame("surface glyph atlas row size is too large".to_owned())
    })?;
    let expected_len = usize::try_from(row_bytes)
        .ok()
        .and_then(|row_bytes| {
            usize::try_from(frame.atlas.height)
                .ok()
                .and_then(|height| row_bytes.checked_mul(height))
        })
        .ok_or_else(|| {
            SurfaceFrameError::InvalidFrame("surface glyph atlas byte size is too large".to_owned())
        })?;
    if frame.atlas.rgba.len() != expected_len {
        return Err(SurfaceFrameError::InvalidFrame(format!(
            "surface glyph atlas has {} bytes, expected {expected_len}",
            frame.atlas.rgba.len()
        )));
    }
    if frame.batch.quads.is_empty() != frame.batch.indices.is_empty() {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph quads and indices must both be present or both be empty".to_owned(),
        ));
    }
    if frame.background_batch.quads.is_empty() != frame.background_batch.indices.is_empty() {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface background quads and indices must both be present or both be empty".to_owned(),
        ));
    }
    if frame.decoration_batch.quads.is_empty() != frame.decoration_batch.indices.is_empty() {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface decoration quads and indices must both be present or both be empty".to_owned(),
        ));
    }
    if frame.cursor_batch.quads.is_empty() != frame.cursor_batch.indices.is_empty() {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface cursor quads and indices must both be present or both be empty".to_owned(),
        ));
    }
    if frame.batch.quads.is_empty()
        && frame.background_batch.quads.is_empty()
        && frame.decoration_batch.quads.is_empty()
        && frame.cursor_batch.quads.is_empty()
    {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph frame requires non-empty glyph, background, decoration, or cursor quads"
                .to_owned(),
        ));
    }
    Ok(SurfaceGlyphAtlasLayout {
        row_bytes,
        expected_len,
    })
}

fn validate_surface_background_buffers(
    vertex_bytes: &[u8],
    index_bytes: &[u8],
    index_count: usize,
) -> std::result::Result<SurfaceGlyphBufferLayout, SurfaceFrameError> {
    if vertex_bytes.is_empty() || index_bytes.is_empty() || index_count == 0 {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface background draw buffers must be non-empty".to_owned(),
        ));
    }
    let vertex_buffer_size = u64::try_from(vertex_bytes.len()).map_err(|_| {
        SurfaceFrameError::InvalidFrame("surface background vertex buffer is too large".to_owned())
    })?;
    let index_buffer_size = u64::try_from(index_bytes.len()).map_err(|_| {
        SurfaceFrameError::InvalidFrame("surface background index buffer is too large".to_owned())
    })?;
    let index_count = u32::try_from(index_count).map_err(|_| {
        SurfaceFrameError::InvalidFrame("surface background index count is too large".to_owned())
    })?;
    Ok(SurfaceGlyphBufferLayout {
        vertex_buffer_size,
        index_buffer_size,
        index_count,
    })
}

fn validate_surface_glyph_buffers(
    vertex_bytes: &[u8],
    index_bytes: &[u8],
    index_count: usize,
) -> std::result::Result<SurfaceGlyphBufferLayout, SurfaceFrameError> {
    if vertex_bytes.is_empty() || index_bytes.is_empty() || index_count == 0 {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph draw buffers must be non-empty".to_owned(),
        ));
    }
    let vertex_buffer_size = u64::try_from(vertex_bytes.len()).map_err(|_| {
        SurfaceFrameError::InvalidFrame("surface glyph vertex buffer is too large".to_owned())
    })?;
    let index_buffer_size = u64::try_from(index_bytes.len()).map_err(|_| {
        SurfaceFrameError::InvalidFrame("surface glyph index buffer is too large".to_owned())
    })?;
    let index_count = u32::try_from(index_count).map_err(|_| {
        SurfaceFrameError::InvalidFrame("surface glyph index count is too large".to_owned())
    })?;
    Ok(SurfaceGlyphBufferLayout {
        vertex_buffer_size,
        index_buffer_size,
        index_count,
    })
}

fn surface_background_vertex_bytes(
    batch: &BackgroundQuadBatch,
    width: u32,
    height: u32,
) -> std::result::Result<Vec<u8>, SurfaceFrameError> {
    if width == 0 || height == 0 {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph frame dimensions must be non-zero".to_owned(),
        ));
    }
    let width = width as f32;
    let height = height as f32;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(surface_background_vertex_byte_capacity(batch.quads.len())?)
        .map_err(|_| {
            SurfaceFrameError::InvalidFrame(
                "surface background vertex bytes are too large to allocate".to_owned(),
            )
        })?;
    for quad in &batch.quads {
        for vertex in quad.vertices {
            let ndc_x = (vertex.position[0] / width * 2.0) - 1.0;
            let ndc_y = 1.0 - (vertex.position[1] / height * 2.0);
            for value in [
                ndc_x,
                ndc_y,
                vertex.color_rgba[0],
                vertex.color_rgba[1],
                vertex.color_rgba[2],
                vertex.color_rgba[3],
            ] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
    Ok(bytes)
}

fn surface_background_vertex_byte_capacity(
    quad_count: usize,
) -> std::result::Result<usize, SurfaceFrameError> {
    quad_count.checked_mul(4 * 6 * 4).ok_or_else(|| {
        SurfaceFrameError::InvalidFrame("surface background vertex bytes are too large".to_owned())
    })
}

fn surface_background_index_bytes(batch: &BackgroundQuadBatch) -> Vec<u8> {
    batch
        .indices
        .iter()
        .flat_map(|index| index.to_le_bytes())
        .collect()
}

fn surface_glyph_vertex_bytes(
    batch: &GlyphQuadBatch,
    width: u32,
    height: u32,
) -> std::result::Result<Vec<u8>, SurfaceFrameError> {
    if width == 0 || height == 0 {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph frame dimensions must be non-zero".to_owned(),
        ));
    }
    let width = width as f32;
    let height = height as f32;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(surface_glyph_vertex_byte_capacity(batch.quads.len())?)
        .map_err(|_| {
            SurfaceFrameError::InvalidFrame(
                "surface glyph vertex bytes are too large to allocate".to_owned(),
            )
        })?;
    for quad in &batch.quads {
        for vertex in quad.vertices {
            let ndc_x = (vertex.position[0] / width * 2.0) - 1.0;
            let ndc_y = 1.0 - (vertex.position[1] / height * 2.0);
            for value in [
                ndc_x,
                ndc_y,
                vertex.uv[0],
                vertex.uv[1],
                vertex.foreground_rgba[0],
                vertex.foreground_rgba[1],
                vertex.foreground_rgba[2],
                vertex.foreground_rgba[3],
            ] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
    Ok(bytes)
}

fn surface_glyph_vertex_byte_capacity(
    quad_count: usize,
) -> std::result::Result<usize, SurfaceFrameError> {
    quad_count.checked_mul(4 * 8 * 4).ok_or_else(|| {
        SurfaceFrameError::InvalidFrame("surface glyph vertex bytes are too large".to_owned())
    })
}

fn surface_glyph_index_bytes(batch: &GlyphQuadBatch) -> Vec<u8> {
    batch
        .indices
        .iter()
        .flat_map(|index| index.to_le_bytes())
        .collect()
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

/// CPU-side render planner for deterministic renderer tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderPlanner {
    font_size_px: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClippedDirtyRegion {
    row_start: u16,
    row_end: u16,
    col_start: u16,
    col_end: u16,
}

impl ClippedDirtyRegion {
    fn rows(self) -> u16 {
        self.row_end - self.row_start
    }

    fn cols(self) -> u16 {
        self.col_end - self.col_start
    }
}

impl RenderPlanner {
    /// Create a render planner for a fixed font size.
    pub fn new(font_size_px: u16) -> Self {
        Self { font_size_px }
    }

    /// Build a deterministic render plan from a terminal snapshot and dirty regions.
    pub fn plan_frame(
        &mut self,
        grid: &GridSnapshot,
        cursor: CursorSnapshot,
        dirty_regions: &[DirtyRegion],
        atlas: &mut GlyphAtlas,
    ) -> Result<RenderPlan> {
        let estimated_dirty_cells = dirty_regions
            .iter()
            .filter_map(|region| clipped_dirty_region(region, grid))
            .map(|region| usize::from(region.rows()) * usize::from(region.cols()))
            .sum();
        let mut glyphs = Vec::with_capacity(estimated_dirty_cells);
        let mut backgrounds = Vec::new();
        let mut decorations = Vec::new();
        for region in dirty_regions {
            let Some(region) = clipped_dirty_region(region, grid) else {
                continue;
            };
            for row in region.row_start..region.row_end {
                for col in region.col_start..region.col_end {
                    let cell = grid.cell(row, col);
                    if let Some(color_rgba8) = style_background_rgba8(cell.style) {
                        append_background_fill(&mut backgrounds, row, col, color_rgba8);
                    }
                    append_cell_decorations(
                        &mut decorations,
                        row,
                        col,
                        cell.style,
                        grid.cell_underline_color(row, col),
                    );
                    if cell.text.is_empty() || cell.is_wide_trailing {
                        continue;
                    }
                    if cell.text.chars().all(char::is_whitespace) {
                        continue;
                    }
                    let Some(ch) = cell.text.chars().next() else {
                        continue;
                    };
                    let text = cell.text.clone();
                    let glyph_key = GlyphKey::for_text(&text, ch, cell.style, self.font_size_px);
                    let atlas_entry = atlas.lookup_or_insert(glyph_key)?;
                    glyphs.push(PlannedGlyph {
                        row,
                        col,
                        text,
                        ch,
                        style: cell.style,
                        font_size_px: self.font_size_px,
                        is_wide: cell.is_wide_leading,
                        atlas_entry,
                    });
                }
            }
        }
        Ok(RenderPlan {
            viewport_cols: grid.cols,
            viewport_rows: grid.rows,
            cursor,
            clear_regions: dirty_regions.to_vec(),
            backgrounds,
            decorations,
            glyphs,
        })
    }
}

fn append_background_fill(
    backgrounds: &mut Vec<PlannedBackground>,
    row: u16,
    col: u16,
    color_rgba8: [u8; 4],
) {
    if let Some(last) = backgrounds.last_mut()
        && last.row == row
        && last.col.saturating_add(last.cols) == col
        && last.color_rgba8 == color_rgba8
    {
        last.cols = last.cols.saturating_add(1);
        return;
    }
    backgrounds.push(PlannedBackground {
        row,
        col,
        cols: 1,
        color_rgba8,
    });
}

fn append_cell_decorations(
    decorations: &mut Vec<PlannedTextDecoration>,
    row: u16,
    col: u16,
    style: Style,
    underline_color: Color,
) {
    if style.hidden {
        return;
    }
    if style.underline {
        match style.underline_style {
            UnderlineStyle::Single => append_text_decoration(
                decorations,
                row,
                col,
                TextDecorationKind::Underline,
                decoration_color_rgba8(underline_color, style),
            ),
            UnderlineStyle::Double => {
                let color_rgba8 = decoration_color_rgba8(underline_color, style);
                append_text_decoration(
                    decorations,
                    row,
                    col,
                    TextDecorationKind::DoubleUnderlineTop,
                    color_rgba8,
                );
                append_text_decoration(
                    decorations,
                    row,
                    col,
                    TextDecorationKind::DoubleUnderlineBottom,
                    color_rgba8,
                );
            }
            UnderlineStyle::Curly => append_text_decoration(
                decorations,
                row,
                col,
                TextDecorationKind::CurlyUnderline,
                decoration_color_rgba8(underline_color, style),
            ),
            UnderlineStyle::Dotted => append_text_decoration(
                decorations,
                row,
                col,
                TextDecorationKind::DottedUnderline,
                decoration_color_rgba8(underline_color, style),
            ),
            UnderlineStyle::Dashed => append_text_decoration(
                decorations,
                row,
                col,
                TextDecorationKind::DashedUnderline,
                decoration_color_rgba8(underline_color, style),
            ),
        }
    }
    if style.overline {
        append_text_decoration(
            decorations,
            row,
            col,
            TextDecorationKind::Overline,
            decoration_color_rgba8(Color::Default, style),
        );
    }
    if style.strikethrough {
        append_text_decoration(
            decorations,
            row,
            col,
            TextDecorationKind::Strikethrough,
            decoration_color_rgba8(Color::Default, style),
        );
    }
}

fn append_text_decoration(
    decorations: &mut Vec<PlannedTextDecoration>,
    row: u16,
    col: u16,
    kind: TextDecorationKind,
    color_rgba8: [u8; 4],
) {
    if let Some(last) = decorations.iter_mut().rev().take(4).find(|last| {
        last.row == row
            && last.col.saturating_add(last.cols) == col
            && last.kind == kind
            && last.color_rgba8 == color_rgba8
    }) {
        last.cols = last.cols.saturating_add(1);
        return;
    }
    decorations.push(PlannedTextDecoration {
        row,
        col,
        cols: 1,
        kind,
        color_rgba8,
    });
}

fn clipped_dirty_region(region: &DirtyRegion, grid: &GridSnapshot) -> Option<ClippedDirtyRegion> {
    let row_start = region.row.min(grid.rows);
    let col_start = region.col.min(grid.cols);
    let row_end = (u32::from(region.row) + u32::from(region.rows)).min(u32::from(grid.rows));
    let col_end = (u32::from(region.col) + u32::from(region.cols)).min(u32::from(grid.cols));
    let row_end = u16::try_from(row_end).ok()?;
    let col_end = u16::try_from(col_end).ok()?;
    if row_start >= row_end || col_start >= col_end {
        return None;
    }
    Some(ClippedDirtyRegion {
        row_start,
        row_end,
        col_start,
        col_end,
    })
}

/// Deterministic CPU-side frame plan consumed by the native renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPlan {
    /// Viewport columns represented by this plan.
    pub viewport_cols: u16,
    /// Viewport rows represented by this plan.
    pub viewport_rows: u16,
    /// Cursor state to draw for this frame.
    pub cursor: CursorSnapshot,
    /// Dirty rectangles to clear before drawing glyphs.
    pub clear_regions: Vec<DirtyRegion>,
    /// Styled cell background fills in row-major order.
    pub backgrounds: Vec<PlannedBackground>,
    /// Styled text-decoration fills in row-major order.
    pub decorations: Vec<PlannedTextDecoration>,
    /// Glyph draw commands in row-major order.
    pub glyphs: Vec<PlannedGlyph>,
}

/// One solid background fill command inside a render plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedBackground {
    /// Grid row.
    pub row: u16,
    /// Starting grid column.
    pub col: u16,
    /// Number of adjacent cells covered by this fill.
    pub cols: u16,
    /// Background color in RGBA8.
    pub color_rgba8: [u8; 4],
}

/// Text-decoration line kind inside a render plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDecorationKind {
    /// Single straight underline.
    Underline,
    /// Upper line of a double straight underline.
    DoubleUnderlineTop,
    /// Lower line of a double straight underline.
    DoubleUnderlineBottom,
    /// Curly underline.
    CurlyUnderline,
    /// Dotted underline.
    DottedUnderline,
    /// Dashed underline.
    DashedUnderline,
    /// Straight overline.
    Overline,
    /// Straight strikethrough.
    Strikethrough,
}

/// One solid text-decoration fill command inside a render plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedTextDecoration {
    /// Grid row.
    pub row: u16,
    /// Starting grid column.
    pub col: u16,
    /// Number of adjacent cells covered by this decoration fill.
    pub cols: u16,
    /// Decoration line kind.
    pub kind: TextDecorationKind,
    /// Decoration color in RGBA8.
    pub color_rgba8: [u8; 4],
}

/// One glyph draw command inside a render plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedGlyph {
    /// Grid row.
    pub row: u16,
    /// Grid column.
    pub col: u16,
    /// Full terminal cell text to draw.
    pub text: String,
    /// Character to draw.
    pub ch: char,
    /// Cell style for the glyph.
    pub style: Style,
    /// Font size used when allocating the glyph atlas entry.
    pub font_size_px: u16,
    /// Whether this glyph occupies two terminal cells.
    pub is_wide: bool,
    /// Glyph atlas handle allocated for this glyph.
    pub atlas_entry: GlyphEntry,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    use crate::terminal::{Terminal, TerminalConfig};

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

    fn empty_grid_snapshot(rows: u16, cols: u16) -> GridSnapshot {
        GridSnapshot {
            rows,
            cols,
            hyperlinks: Vec::new(),
            underline_colors: Vec::new(),
            cells: Vec::new(),
        }
    }

    #[test]
    fn clipped_dirty_region_uses_widened_bounds_at_u16_edges() {
        let grid = empty_grid_snapshot(u16::MAX, u16::MAX);
        let region = DirtyRegion {
            row: u16::MAX - 1,
            col: u16::MAX - 2,
            rows: 8,
            cols: 9,
        };

        assert_eq!(
            clipped_dirty_region(&region, &grid),
            Some(ClippedDirtyRegion {
                row_start: u16::MAX - 1,
                row_end: u16::MAX,
                col_start: u16::MAX - 2,
                col_end: u16::MAX,
            })
        );
    }

    #[test]
    fn clipped_dirty_region_rejects_regions_outside_grid() {
        let grid = empty_grid_snapshot(10, 10);
        let region = DirtyRegion {
            row: 12,
            col: 0,
            rows: 1,
            cols: 1,
        };

        assert_eq!(clipped_dirty_region(&region, &grid), None);
    }

    #[test]
    fn render_planner_ignores_dirty_regions_outside_grid() {
        let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());
        terminal.write_str("AB").unwrap();
        let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
        let mut planner = RenderPlanner::new(14);
        let dirty = [DirtyRegion {
            row: 4,
            col: 0,
            rows: 1,
            cols: 1,
        }];

        let plan = planner
            .plan_frame(
                &terminal.dump_grid(),
                terminal.dump_cursor(),
                &dirty,
                &mut atlas,
            )
            .unwrap();

        assert!(plan.glyphs.is_empty());
        assert_eq!(atlas.metrics().entries, 0);
    }

    #[test]
    fn atlas_columns_for_glyphs_uses_widened_slot_math() {
        let glyphs = [
            GlyphBitmap {
                entry: GlyphEntry {
                    slot: 0,
                    generation: 0,
                },
                width: 1,
                height: 1,
                rgba: Vec::new(),
            },
            GlyphBitmap {
                entry: GlyphEntry {
                    slot: 3,
                    generation: 0,
                },
                width: 1,
                height: 1,
                rgba: Vec::new(),
            },
        ];

        assert_eq!(atlas_columns_for_glyphs(&glyphs), 2);
    }

    #[test]
    fn atlas_columns_for_glyphs_handles_maximum_slot_without_overflow() {
        let glyphs = [GlyphBitmap {
            entry: GlyphEntry {
                slot: u32::MAX,
                generation: 0,
            },
            width: 1,
            height: 1,
            rgba: Vec::new(),
        }];

        assert_eq!(atlas_columns_for_glyphs(&glyphs), 65_536);
    }

    #[test]
    fn prepared_surface_glyph_frame_rejects_overflowing_pixel_width() {
        let entry = GlyphEntry {
            slot: 0,
            generation: 0,
        };
        let plan = RenderPlan {
            viewport_cols: 2,
            viewport_rows: 1,
            cursor: CursorSnapshot {
                row: 0,
                col: 0,
                visible: true,
                shape: crate::terminal::CursorShape::Block,
                blinking: true,
            },
            clear_regions: Vec::new(),
            backgrounds: Vec::new(),
            decorations: Vec::new(),
            glyphs: vec![PlannedGlyph {
                row: 0,
                col: 0,
                text: "A".to_owned(),
                ch: 'A',
                style: Style::default(),
                font_size_px: 14,
                is_wide: false,
                atlas_entry: entry,
            }],
        };
        let glyphs = [GlyphBitmap {
            entry,
            width: u32::MAX,
            height: 1,
            rgba: Vec::new(),
        }];

        let error =
            PreparedSurfaceGlyphFrame::from_render_plan(&plan, &glyphs, [0.0, 0.0, 0.0, 1.0])
                .unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame(
                "surface glyph frame width is too large to represent".to_owned()
            )
        );
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
