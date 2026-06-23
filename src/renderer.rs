//! GPU renderer boundary.

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use thiserror::Error;

use crate::cell::{Color, Style, UnderlineStyle};
use crate::config::{GromaqConfig, MAX_TARGET_FPS};
use crate::dirty::DirtyRegion;
use crate::error::{GromaqError, Result};
use crate::grid::GridSnapshot;
use crate::terminal::CursorSnapshot;

const NANOS_PER_SECOND: u64 = 1_000_000_000;
const DEFAULT_RENDERER_FONT_SIZE_PX: u16 = 14;
const DEFAULT_GLYPH_ATLAS_CAPACITY: usize = 4096;
const MAX_GLYPH_ATLAS_CAPACITY: usize = 65_536;

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

/// Errors produced when choosing a native `wgpu` surface configuration.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SurfaceConfigError {
    /// Surface dimensions must be non-zero.
    #[error("surface dimensions must be non-zero, got {width}x{height}")]
    InvalidSize {
        /// Requested surface width.
        width: u32,
        /// Requested surface height.
        height: u32,
    },
    /// The surface reported no supported texture formats.
    #[error("surface reported no supported texture formats")]
    NoSupportedFormats,
    /// The surface reported no supported presentation modes.
    #[error("surface reported no supported presentation modes")]
    NoSupportedPresentModes,
    /// The surface reported no supported alpha modes.
    #[error("surface reported no supported alpha modes")]
    NoSupportedAlphaModes,
    /// The surface cannot be rendered into.
    #[error("surface does not support render attachment usage")]
    MissingRenderAttachmentUsage,
}

/// Chooses deterministic `wgpu` surface configurations for the native renderer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SurfaceConfigPlanner;

impl SurfaceConfigPlanner {
    /// Create a surface configuration planner.
    pub fn new() -> Self {
        Self
    }

    /// Build a `wgpu` surface configuration from adapter/surface capabilities.
    pub fn plan(
        &self,
        capabilities: &wgpu::SurfaceCapabilities,
        width: u32,
        height: u32,
    ) -> std::result::Result<wgpu::SurfaceConfiguration, SurfaceConfigError> {
        if width == 0 || height == 0 {
            return Err(SurfaceConfigError::InvalidSize { width, height });
        }
        if capabilities.formats.is_empty() {
            return Err(SurfaceConfigError::NoSupportedFormats);
        }
        if capabilities.present_modes.is_empty() {
            return Err(SurfaceConfigError::NoSupportedPresentModes);
        }
        if capabilities.alpha_modes.is_empty() {
            return Err(SurfaceConfigError::NoSupportedAlphaModes);
        }
        if !capabilities
            .usages
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
        {
            return Err(SurfaceConfigError::MissingRenderAttachmentUsage);
        }

        Ok(wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: choose_surface_format(&capabilities.formats),
            width,
            height,
            present_mode: choose_present_mode(&capabilities.present_modes),
            desired_maximum_frame_latency: 1,
            alpha_mode: choose_alpha_mode(&capabilities.alpha_modes),
            view_formats: Vec::new(),
        })
    }
}

/// Platform action required after a surface lifecycle transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceLifecycleAction {
    /// No surface action is required.
    None,
    /// Configure the surface for the first time.
    Configure,
    /// Reconfigure an already-created surface.
    Reconfigure,
    /// Defer configuration while the window is minimized or otherwise zero-sized.
    DeferZeroSize,
}

/// Surface endpoint that can receive an executable `wgpu` surface configuration.
pub trait SurfaceBackend {
    /// Apply `config` to the native surface boundary.
    fn configure(&mut self, config: &wgpu::SurfaceConfiguration);
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
    /// Textured glyph quads and indices to draw.
    pub batch: &'a GlyphQuadBatch,
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
    batch: GlyphQuadBatch,
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
                    .map_err(SurfaceFrameError::InvalidFrame)
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let columns = atlas_columns_for_glyphs(&padded);
        let atlas = GlyphAtlasImage::pack_rgba8(slot_width, slot_height, columns, &padded)
            .map_err(SurfaceFrameError::InvalidFrame)?;
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
        Ok(Self {
            atlas,
            batch,
            width,
            height,
            clear_color,
        })
    }

    /// Borrow this owned frame as a surface presentation frame.
    pub fn as_surface_glyph_frame(&self) -> SurfaceGlyphFrame<'_> {
        SurfaceGlyphFrame {
            atlas: &self.atlas,
            batch: &self.batch,
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
                array_stride: 16,
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
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..buffer_layout.index_count, 0, 0..1);
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
    if frame.batch.quads.is_empty() || frame.batch.indices.is_empty() {
        return Err(SurfaceFrameError::InvalidFrame(
            "surface glyph frame requires non-empty quads and indices".to_owned(),
        ));
    }
    Ok(SurfaceGlyphAtlasLayout {
        row_bytes,
        expected_len,
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
            for value in [ndc_x, ndc_y, vertex.uv[0], vertex.uv[1]] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
    Ok(bytes)
}

fn surface_glyph_vertex_byte_capacity(
    quad_count: usize,
) -> std::result::Result<usize, SurfaceFrameError> {
    quad_count.checked_mul(4 * 4 * 4).ok_or_else(|| {
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

const SURFACE_GLYPH_WGSL: &str = r#"
struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var output: VertexOut;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    return output;
}

@group(0) @binding(0) var atlas_texture: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(atlas_texture, atlas_sampler, input.uv);
}
"#;

/// Deterministic state for native `wgpu` surface configuration and resize handling.
#[derive(Debug, Clone)]
pub struct SurfaceLifecycle {
    planner: SurfaceConfigPlanner,
    current_config: Option<wgpu::SurfaceConfiguration>,
    current_size: Option<(u32, u32)>,
    suspended_for_zero_size: bool,
    configure_count: u64,
}

impl SurfaceLifecycle {
    /// Create surface lifecycle state using `planner`.
    pub fn new(planner: SurfaceConfigPlanner) -> Self {
        Self {
            planner,
            current_config: None,
            current_size: None,
            suspended_for_zero_size: false,
            configure_count: 0,
        }
    }

    /// Configure the surface for an initial non-zero size.
    pub fn configure(
        &mut self,
        capabilities: &wgpu::SurfaceCapabilities,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError> {
        self.apply_size(capabilities, width, height)
    }

    /// Handle a native window resize.
    pub fn on_resized(
        &mut self,
        capabilities: &wgpu::SurfaceCapabilities,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError> {
        self.apply_size(capabilities, width, height)
    }

    /// Return the current surface configuration.
    pub fn current_config(&self) -> Option<&wgpu::SurfaceConfiguration> {
        self.current_config.as_ref()
    }

    /// Return the current non-zero surface size.
    pub fn size(&self) -> Option<(u32, u32)> {
        self.current_size
    }

    /// Whether a valid surface configuration exists.
    pub fn is_configured(&self) -> bool {
        self.current_config.is_some()
    }

    /// Whether configuration is suspended because the window is zero-sized.
    pub fn is_suspended(&self) -> bool {
        self.suspended_for_zero_size
    }

    /// Number of surface configuration transitions applied.
    pub fn configure_count(&self) -> u64 {
        self.configure_count
    }

    fn apply_size(
        &mut self,
        capabilities: &wgpu::SurfaceCapabilities,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError> {
        if width == 0 || height == 0 {
            self.suspended_for_zero_size = true;
            return Ok(SurfaceLifecycleAction::DeferZeroSize);
        }
        let config = self.planner.plan(capabilities, width, height)?;
        let action = if self.current_config.is_some() {
            if self.current_size == Some((width, height)) && !self.suspended_for_zero_size {
                SurfaceLifecycleAction::None
            } else {
                SurfaceLifecycleAction::Reconfigure
            }
        } else {
            SurfaceLifecycleAction::Configure
        };

        if action != SurfaceLifecycleAction::None {
            self.configure_count += 1;
        }
        self.current_size = Some((width, height));
        self.current_config = Some(config);
        self.suspended_for_zero_size = false;
        Ok(action)
    }
}

/// Applies planned surface lifecycle transitions to a concrete surface backend.
#[derive(Debug, Clone)]
pub struct SurfaceConfigurationController {
    lifecycle: SurfaceLifecycle,
}

impl SurfaceConfigurationController {
    /// Create a surface configuration controller.
    pub fn new(planner: SurfaceConfigPlanner) -> Self {
        Self {
            lifecycle: SurfaceLifecycle::new(planner),
        }
    }

    /// Access the underlying lifecycle state.
    pub fn lifecycle(&self) -> &SurfaceLifecycle {
        &self.lifecycle
    }

    /// Configure an initial surface size and apply the resulting config to `backend`.
    pub fn configure<B>(
        &mut self,
        backend: &mut B,
        capabilities: &wgpu::SurfaceCapabilities,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError>
    where
        B: SurfaceBackend,
    {
        let action = self.lifecycle.configure(capabilities, width, height)?;
        self.apply_action(backend, action);
        Ok(action)
    }

    /// Resize a configured surface and apply reconfiguration to `backend` when needed.
    pub fn resize<B>(
        &mut self,
        backend: &mut B,
        capabilities: &wgpu::SurfaceCapabilities,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError>
    where
        B: SurfaceBackend,
    {
        let action = self.lifecycle.on_resized(capabilities, width, height)?;
        self.apply_action(backend, action);
        Ok(action)
    }

    fn apply_action<B>(&self, backend: &mut B, action: SurfaceLifecycleAction)
    where
        B: SurfaceBackend,
    {
        if matches!(
            action,
            SurfaceLifecycleAction::Configure | SurfaceLifecycleAction::Reconfigure
        ) && let Some(config) = self.lifecycle.current_config()
        {
            backend.configure(config);
        }
    }
}

fn choose_surface_format(formats: &[wgpu::TextureFormat]) -> wgpu::TextureFormat {
    [
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ]
    .into_iter()
    .find(|preferred| formats.contains(preferred))
    .unwrap_or_else(|| {
        formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(formats[0])
    })
}

fn choose_present_mode(present_modes: &[wgpu::PresentMode]) -> wgpu::PresentMode {
    if present_modes.contains(&wgpu::PresentMode::Fifo) {
        wgpu::PresentMode::Fifo
    } else {
        present_modes[0]
    }
}

fn choose_alpha_mode(alpha_modes: &[wgpu::CompositeAlphaMode]) -> wgpu::CompositeAlphaMode {
    if alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
        wgpu::CompositeAlphaMode::Opaque
    } else {
        alpha_modes[0]
    }
}

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
        for region in dirty_regions {
            let Some(region) = clipped_dirty_region(region, grid) else {
                continue;
            };
            for row in region.row_start..region.row_end {
                for col in region.col_start..region.col_end {
                    let cell = grid.cell(row, col);
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
            glyphs,
        })
    }
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
    /// Glyph draw commands in row-major order.
    pub glyphs: Vec<PlannedGlyph>,
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

/// Pixel and atlas layout used to build textured glyph quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphQuadConfig {
    /// Terminal cell width in pixels.
    pub cell_width_px: u32,
    /// Terminal cell height in pixels.
    pub cell_height_px: u32,
    /// Glyph atlas slot width in pixels.
    pub atlas_slot_width_px: u32,
    /// Glyph atlas slot height in pixels.
    pub atlas_slot_height_px: u32,
    /// Number of atlas slots per row.
    pub atlas_columns: u32,
    /// Atlas texture width in pixels.
    pub atlas_width_px: u32,
    /// Atlas texture height in pixels.
    pub atlas_height_px: u32,
}

/// Errors produced while building textured glyph quads.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GlyphQuadError {
    /// Pixel or atlas dimensions must be non-zero.
    #[error("glyph quad dimensions must be non-zero")]
    ZeroDimension,
    /// The planned glyph batch cannot be represented in `u32` GPU indices.
    #[error("glyph quad count is too large for u32 GPU indices")]
    IndexCountTooLarge,
    /// A glyph atlas slot falls outside the configured atlas texture.
    #[error("glyph atlas slot {slot} is outside the configured atlas image")]
    SlotOutsideAtlas {
        /// Atlas slot index that could not be represented.
        slot: u32,
    },
}

/// One vertex for a textured glyph quad.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphVertex {
    /// Pixel-space output position.
    pub position: [f32; 2],
    /// Atlas texture coordinate.
    pub uv: [f32; 2],
}

/// One textured glyph quad derived from a planned glyph.
#[derive(Debug, Clone, PartialEq)]
pub struct GlyphQuad {
    /// Full terminal cell text represented by this quad.
    pub text: String,
    /// Character represented by this quad.
    pub ch: char,
    /// Atlas entry sampled by this quad.
    pub atlas_entry: GlyphEntry,
    /// Quad vertices in top-left, top-right, bottom-right, bottom-left order.
    pub vertices: [GlyphVertex; 4],
}

/// Indexed glyph quad batch ready for GPU vertex/index buffer upload.
#[derive(Debug, Clone, PartialEq)]
pub struct GlyphQuadBatch {
    /// Textured glyph quads.
    pub quads: Vec<GlyphQuad>,
    /// Triangle indices for all quads.
    pub indices: Vec<u32>,
}

/// Deterministic CPU-side planner for terminal glyph draw quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphQuadPlanner {
    config: GlyphQuadConfig,
}

impl GlyphQuadPlanner {
    /// Create a glyph quad planner.
    pub fn new(config: GlyphQuadConfig) -> Self {
        Self { config }
    }

    /// Build textured quads and triangle indices from a render plan.
    pub fn plan(&self, plan: &RenderPlan) -> std::result::Result<GlyphQuadBatch, GlyphQuadError> {
        self.validate_config()?;
        let mut quads = Vec::new();
        quads
            .try_reserve_exact(plan.glyphs.len())
            .map_err(|_| GlyphQuadError::IndexCountTooLarge)?;
        let mut indices = Vec::new();
        indices
            .try_reserve_exact(checked_glyph_quad_index_capacity(plan.glyphs.len())?)
            .map_err(|_| GlyphQuadError::IndexCountTooLarge)?;

        for glyph in &plan.glyphs {
            let quad = self.plan_glyph(glyph)?;
            let base = checked_glyph_quad_base_index(quads.len())?;
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            quads.push(quad);
        }

        Ok(GlyphQuadBatch { quads, indices })
    }

    fn validate_config(&self) -> std::result::Result<(), GlyphQuadError> {
        if self.config.cell_width_px == 0
            || self.config.cell_height_px == 0
            || self.config.atlas_slot_width_px == 0
            || self.config.atlas_slot_height_px == 0
            || self.config.atlas_columns == 0
            || self.config.atlas_width_px == 0
            || self.config.atlas_height_px == 0
        {
            return Err(GlyphQuadError::ZeroDimension);
        }
        Ok(())
    }

    fn plan_glyph(&self, glyph: &PlannedGlyph) -> std::result::Result<GlyphQuad, GlyphQuadError> {
        let cell_width = self.config.cell_width_px as f32;
        let cell_height = self.config.cell_height_px as f32;
        let x0 = f32::from(glyph.col) * cell_width;
        let y0 = f32::from(glyph.row) * cell_height;
        let glyph_cells = if glyph.is_wide { 2.0 } else { 1.0 };
        let x1 = x0 + (cell_width * glyph_cells);
        let y1 = y0 + cell_height;

        let slot = glyph.atlas_entry.slot;
        let slot_col = slot % self.config.atlas_columns;
        let slot_row = slot / self.config.atlas_columns;
        let atlas_x0 = slot_col
            .checked_mul(self.config.atlas_slot_width_px)
            .ok_or(GlyphQuadError::SlotOutsideAtlas { slot })?;
        let atlas_y0 = slot_row
            .checked_mul(self.config.atlas_slot_height_px)
            .ok_or(GlyphQuadError::SlotOutsideAtlas { slot })?;
        let atlas_x1 = atlas_x0
            .checked_add(self.config.atlas_slot_width_px)
            .ok_or(GlyphQuadError::SlotOutsideAtlas { slot })?;
        let atlas_y1 = atlas_y0
            .checked_add(self.config.atlas_slot_height_px)
            .ok_or(GlyphQuadError::SlotOutsideAtlas { slot })?;
        if atlas_x1 > self.config.atlas_width_px || atlas_y1 > self.config.atlas_height_px {
            return Err(GlyphQuadError::SlotOutsideAtlas { slot });
        }

        let u0 = atlas_x0 as f32 / self.config.atlas_width_px as f32;
        let v0 = atlas_y0 as f32 / self.config.atlas_height_px as f32;
        let u1 = atlas_x1 as f32 / self.config.atlas_width_px as f32;
        let v1 = atlas_y1 as f32 / self.config.atlas_height_px as f32;

        Ok(GlyphQuad {
            text: glyph.text.clone(),
            ch: glyph.ch,
            atlas_entry: glyph.atlas_entry,
            vertices: [
                GlyphVertex {
                    position: [x0, y0],
                    uv: [u0, v0],
                },
                GlyphVertex {
                    position: [x1, y0],
                    uv: [u1, v0],
                },
                GlyphVertex {
                    position: [x1, y1],
                    uv: [u1, v1],
                },
                GlyphVertex {
                    position: [x0, y1],
                    uv: [u0, v1],
                },
            ],
        })
    }
}

fn checked_glyph_quad_base_index(quad_index: usize) -> std::result::Result<u32, GlyphQuadError> {
    u32::try_from(quad_index)
        .ok()
        .and_then(|index| index.checked_mul(4))
        .ok_or(GlyphQuadError::IndexCountTooLarge)
}

fn checked_glyph_quad_index_capacity(
    quad_count: usize,
) -> std::result::Result<usize, GlyphQuadError> {
    quad_count
        .checked_mul(6)
        .ok_or(GlyphQuadError::IndexCountTooLarge)
}

/// Reason a frame decision was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderReason {
    /// No work is pending.
    Idle,
    /// First frame after dirty state appears.
    FirstDirtyFrame,
    /// Dirty state is pending and the frame interval has elapsed.
    Dirty,
    /// Dirty state exists but the scheduler is waiting for the next frame boundary.
    FramePaced,
}

/// Deterministic frame-scheduling decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDecision {
    /// Whether the renderer should draw now.
    pub should_render: bool,
    /// Optional wait duration before rendering should be reconsidered.
    pub wait_for: Option<Duration>,
    /// Decision reason.
    pub reason: RenderReason,
}

impl FrameDecision {
    /// Build a render-now decision.
    pub fn render(reason: RenderReason) -> Self {
        Self {
            should_render: true,
            wait_for: None,
            reason,
        }
    }

    /// Build an idle decision.
    pub fn idle() -> Self {
        Self {
            should_render: false,
            wait_for: None,
            reason: RenderReason::Idle,
        }
    }

    fn wait(wait_for: Duration) -> Self {
        Self {
            should_render: false,
            wait_for: Some(wait_for),
            reason: RenderReason::FramePaced,
        }
    }
}

/// Frame pacing metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameSchedulerMetrics {
    /// Number of frames marked as presented.
    pub frames_presented: u64,
    /// Number of frame intervals missed between presented frames.
    pub dropped_frames: u64,
}

/// Deterministic frame scheduler for render-loop tests and native UI integration.
#[derive(Debug, Clone)]
pub struct FrameScheduler {
    target_interval: Duration,
    last_presented: Option<Instant>,
    metrics: FrameSchedulerMetrics,
}

impl FrameScheduler {
    /// Create a frame scheduler for `target_fps`.
    pub fn new(target_fps: u32) -> Result<Self> {
        if !(1..=MAX_TARGET_FPS).contains(&target_fps) {
            return Err(GromaqError::InvalidTargetFps {
                minimum: 1,
                maximum: MAX_TARGET_FPS,
                actual: target_fps,
            });
        }
        Ok(Self {
            target_interval: Duration::from_nanos(NANOS_PER_SECOND / u64::from(target_fps)),
            last_presented: None,
            metrics: FrameSchedulerMetrics::default(),
        })
    }

    /// Target interval between presented frames.
    pub fn target_interval(&self) -> Duration {
        self.target_interval
    }

    /// Decide whether a frame should be rendered at `now`.
    pub fn decide(&self, now: Instant, has_dirty: bool) -> FrameDecision {
        if !has_dirty {
            return FrameDecision::idle();
        }
        let Some(last_presented) = self.last_presented else {
            return FrameDecision::render(RenderReason::FirstDirtyFrame);
        };
        let elapsed = now.saturating_duration_since(last_presented);
        if elapsed >= self.target_interval {
            FrameDecision::render(RenderReason::Dirty)
        } else {
            FrameDecision::wait(self.target_interval - elapsed)
        }
    }

    /// Record that a frame was presented at `presented_at`.
    pub fn record_presented(&mut self, presented_at: Instant) {
        if let Some(last_presented) = self.last_presented {
            let elapsed = presented_at.saturating_duration_since(last_presented);
            let intervals = elapsed.as_nanos() / self.target_interval.as_nanos();
            if intervals > 1 {
                self.metrics.dropped_frames = self
                    .metrics
                    .dropped_frames
                    .saturating_add(saturating_u128_to_u64(intervals - 1));
            }
        }
        self.last_presented = Some(presented_at);
        self.metrics.frames_presented = self.metrics.frames_presented.saturating_add(1);
    }

    /// Return scheduler metrics.
    pub fn metrics(&self) -> FrameSchedulerMetrics {
        self.metrics
    }
}

fn saturating_u128_to_u64(value: u128) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// Glyph atlas configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphAtlasConfig {
    capacity: usize,
}

impl GlyphAtlasConfig {
    /// Create a glyph atlas configuration.
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 || capacity > MAX_GLYPH_ATLAS_CAPACITY {
            return Err(GromaqError::InvalidGlyphAtlasCapacity {
                minimum: 1,
                maximum: MAX_GLYPH_ATLAS_CAPACITY,
                actual: capacity,
            });
        }
        Ok(Self { capacity })
    }

    /// Maximum cached glyph entries.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

/// Stable glyph cache text identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GlyphKeyText {
    /// A single scalar value.
    Scalar(char),
    /// A multi-scalar terminal cell text cluster.
    Cluster(String),
}

/// Stable glyph cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    /// Text to render.
    pub text: GlyphKeyText,
    /// Cell style.
    pub style: Style,
    /// Font size in pixels.
    pub font_size_px: u16,
}

impl GlyphKey {
    /// Build a glyph cache key.
    pub fn new(ch: char, style: Style, font_size_px: u16) -> Self {
        Self {
            text: GlyphKeyText::Scalar(ch),
            style: glyph_raster_style(style),
            font_size_px,
        }
    }

    /// Build a glyph cache key for a full terminal cell text cluster.
    pub fn for_text(text: &str, first_char: char, style: Style, font_size_px: u16) -> Self {
        if text.len() == first_char.len_utf8() {
            Self::new(first_char, style, font_size_px)
        } else {
            Self {
                text: GlyphKeyText::Cluster(text.to_owned()),
                style: glyph_raster_style(style),
                font_size_px,
            }
        }
    }
}

fn glyph_raster_style(style: Style) -> Style {
    Style {
        foreground: Color::Default,
        background: Color::Default,
        dim: false,
        underline: false,
        underline_style: UnderlineStyle::Single,
        underline_color_id: 0,
        blink: false,
        hidden: false,
        inverse: false,
        overline: false,
        strikethrough: false,
        ..style
    }
}

/// Glyph atlas entry handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphEntry {
    /// Stable slot index inside the atlas.
    pub slot: u32,
    /// Generation increments whenever a slot is reused.
    pub generation: u64,
}

/// One rasterized glyph bitmap ready for atlas packing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphBitmap {
    /// Atlas entry this bitmap belongs to.
    pub entry: GlyphEntry,
    /// Bitmap width in pixels.
    pub width: u32,
    /// Bitmap height in pixels.
    pub height: u32,
    /// Dense RGBA8 pixels in row-major order.
    pub rgba: Vec<u8>,
}

impl GlyphBitmap {
    /// Try to build a solid RGBA8 glyph bitmap without panicking on oversized dimensions.
    pub fn try_solid_rgba8(
        entry: GlyphEntry,
        width: u32,
        height: u32,
        rgba: [u8; 4],
    ) -> std::result::Result<Self, String> {
        let pixel_count = rgba_pixel_count(width, height)?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(rgba_byte_len(width, height)?)
            .map_err(|_| "solid glyph bitmap is too large to allocate".to_owned())?;
        for _ in 0..pixel_count {
            pixels.extend_from_slice(&rgba);
        }
        Ok(Self {
            entry,
            width,
            height,
            rgba: pixels,
        })
    }

    /// Build a solid RGBA8 glyph bitmap for deterministic renderer tests.
    pub fn solid_rgba8(entry: GlyphEntry, width: u32, height: u32, rgba: [u8; 4]) -> Self {
        Self::try_solid_rgba8(entry, width, height, rgba)
            .expect("deterministic solid glyph bitmap dimensions are valid")
    }

    /// Return this glyph copied into the top-left of a larger transparent bitmap.
    pub fn padded_to(
        &self,
        target_width: u32,
        target_height: u32,
    ) -> std::result::Result<Self, String> {
        if target_width < self.width || target_height < self.height {
            return Err(format!(
                "target {target_width}x{target_height} is smaller than glyph {}x{}",
                self.width, self.height
            ));
        }
        if self.width == target_width && self.height == target_height {
            return Ok(self.clone());
        }

        let source_row_bytes = rgba_row_byte_len(self.width)?;
        let target_row_bytes = rgba_row_byte_len(target_width)?;
        let expected_source_len = rgba_byte_len(self.width, self.height)?;
        if self.rgba.len() != expected_source_len {
            return Err(format!(
                "glyph slot {} expected {expected_source_len} rgba bytes before padding",
                self.entry.slot
            ));
        }

        let source_height = usize::try_from(self.height)
            .map_err(|_| "rgba image dimensions are too large".to_owned())?;
        let mut rgba = zeroed_rgba_buffer(target_width, target_height)?;
        for row in 0..source_height {
            let source_start = checked_rgba_row_offset(row, source_row_bytes)?;
            let target_start = checked_rgba_row_offset(row, target_row_bytes)?;
            rgba[target_start..target_start + source_row_bytes]
                .copy_from_slice(&self.rgba[source_start..source_start + source_row_bytes]);
        }

        Ok(Self {
            entry: self.entry,
            width: target_width,
            height: target_height,
            rgba,
        })
    }
}

fn rgba_row_byte_len(width: u32) -> std::result::Result<usize, String> {
    usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(4))
        .ok_or_else(|| "rgba row dimensions are too large".to_owned())
}

fn rgba_pixel_count(width: u32, height: u32) -> std::result::Result<usize, String> {
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or_else(|| "rgba image dimensions are too large".to_owned())
}

fn rgba_byte_len(width: u32, height: u32) -> std::result::Result<usize, String> {
    rgba_pixel_count(width, height)?
        .checked_mul(4)
        .ok_or_else(|| "rgba image dimensions are too large".to_owned())
}

fn checked_rgba_row_offset(row: usize, row_bytes: usize) -> std::result::Result<usize, String> {
    row.checked_mul(row_bytes)
        .ok_or_else(|| "rgba row offset is too large".to_owned())
}

fn zeroed_rgba_buffer(width: u32, height: u32) -> std::result::Result<Vec<u8>, String> {
    let len = rgba_byte_len(width, height)?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(len)
        .map_err(|_| "rgba image buffer is too large to allocate".to_owned())?;
    rgba.resize(len, 0);
    Ok(rgba)
}

fn rgba_offset(width: u32, x: u32, y: u32) -> std::result::Result<usize, String> {
    usize::try_from(y)
        .ok()
        .and_then(|y| {
            usize::try_from(width)
                .ok()
                .and_then(|width| y.checked_mul(width))
        })
        .and_then(|row_start| {
            usize::try_from(x)
                .ok()
                .and_then(|x| row_start.checked_add(x))
        })
        .and_then(|pixel_offset| pixel_offset.checked_mul(4))
        .ok_or_else(|| "rgba image offset is too large".to_owned())
}

/// Packed RGBA8 glyph atlas image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphAtlasImage {
    /// Atlas image width in pixels.
    pub width: u32,
    /// Atlas image height in pixels.
    pub height: u32,
    /// Dense RGBA8 pixels in row-major order.
    pub rgba: Vec<u8>,
    /// Number of populated atlas slots.
    pub occupied_slots: usize,
}

impl GlyphAtlasImage {
    /// Pack fixed-size RGBA8 glyph bitmaps into slots.
    pub fn pack_rgba8(
        slot_width: u32,
        slot_height: u32,
        columns: u32,
        glyphs: &[GlyphBitmap],
    ) -> std::result::Result<Self, String> {
        if slot_width == 0 || slot_height == 0 || columns == 0 {
            return Err("slot dimensions and columns must be non-zero".to_owned());
        }
        let max_slot = glyphs
            .iter()
            .map(|glyph| glyph.entry.slot)
            .max()
            .unwrap_or(0);
        let rows = (max_slot / columns) + 1;
        let width = slot_width
            .checked_mul(columns)
            .ok_or_else(|| "glyph atlas width is too large".to_owned())?;
        let height = slot_height
            .checked_mul(rows)
            .ok_or_else(|| "glyph atlas height is too large".to_owned())?;
        let mut rgba = zeroed_rgba_buffer(width, height)?;

        for glyph in glyphs {
            let expected_len = rgba_byte_len(slot_width, slot_height)?;
            if glyph.width != slot_width
                || glyph.height != slot_height
                || glyph.rgba.len() != expected_len
            {
                return Err(format!(
                    "glyph slot {} expected {expected_len} rgba bytes for {slot_width}x{slot_height}",
                    glyph.entry.slot
                ));
            }

            let slot_col = glyph.entry.slot % columns;
            let slot_row = glyph.entry.slot / columns;
            for y in 0..slot_height {
                let atlas_y = slot_row
                    .checked_mul(slot_height)
                    .and_then(|row_start| row_start.checked_add(y))
                    .ok_or_else(|| "glyph atlas row offset is too large".to_owned())?;
                let atlas_x = slot_col
                    .checked_mul(slot_width)
                    .ok_or_else(|| "glyph atlas column offset is too large".to_owned())?;
                let atlas_start = rgba_offset(width, atlas_x, atlas_y)?;
                let glyph_start = rgba_offset(slot_width, 0, y)?;
                let row_bytes = rgba_row_byte_len(slot_width)?;
                rgba[atlas_start..atlas_start + row_bytes]
                    .copy_from_slice(&glyph.rgba[glyph_start..glyph_start + row_bytes]);
            }
        }

        Ok(Self {
            width,
            height,
            rgba,
            occupied_slots: glyphs.len(),
        })
    }

    /// Build a deterministic two-slot atlas image for GPU upload smoke tests.
    pub fn smoke_rgba8() -> Self {
        let red = GlyphBitmap::solid_rgba8(
            GlyphEntry {
                slot: 0,
                generation: 0,
            },
            2,
            2,
            [255, 0, 0, 255],
        );
        let green = GlyphBitmap::solid_rgba8(
            GlyphEntry {
                slot: 1,
                generation: 0,
            },
            2,
            2,
            [0, 255, 0, 255],
        );
        Self::pack_rgba8(2, 2, 2, &[red, green]).expect("smoke atlas bitmap dimensions are valid")
    }
}

/// Glyph atlas cache metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GlyphAtlasMetrics {
    /// Cache hits.
    pub hits: u64,
    /// Cache misses.
    pub misses: u64,
    /// Cache evictions.
    pub evictions: u64,
    /// Current entry count.
    pub entries: usize,
}

#[derive(Debug, Clone, Copy)]
struct GlyphSlot {
    entry: GlyphEntry,
}

/// Deterministic glyph atlas cache.
#[derive(Debug)]
pub struct GlyphAtlas {
    config: GlyphAtlasConfig,
    entries: HashMap<GlyphKey, GlyphEntry>,
    lru: VecDeque<GlyphKey>,
    free_slots: Vec<u32>,
    generations: Vec<u64>,
    metrics: GlyphAtlasMetrics,
}

impl GlyphAtlas {
    /// Create an empty glyph atlas.
    pub fn new(config: GlyphAtlasConfig) -> Self {
        let mut free_slots = Vec::with_capacity(config.capacity());
        for slot in (0..config.capacity()).rev() {
            free_slots.push(slot as u32);
        }
        Self {
            config,
            entries: HashMap::with_capacity(config.capacity()),
            lru: VecDeque::with_capacity(config.capacity()),
            free_slots,
            generations: vec![0; config.capacity()],
            metrics: GlyphAtlasMetrics::default(),
        }
    }

    /// Look up a glyph entry or allocate one.
    pub fn lookup_or_insert(&mut self, key: GlyphKey) -> Result<GlyphEntry> {
        if let Some(entry) = self.entries.get(&key).copied() {
            self.metrics.hits += 1;
            self.touch(key);
            return Ok(entry);
        }

        self.metrics.misses += 1;
        let entry = match self.free_slots.pop() {
            Some(slot) => GlyphEntry {
                slot,
                generation: self.generations[slot as usize],
            },
            None => {
                let evicted = self.evict_lru()?;
                let slot = evicted.entry.slot;
                let generation = evicted.entry.generation + 1;
                self.generations[slot as usize] = generation;
                GlyphEntry { slot, generation }
            }
        };
        self.entries.insert(key.clone(), entry);
        self.lru.push_back(key);
        self.metrics.entries = self.entries.len();
        Ok(entry)
    }

    /// Return glyph atlas metrics.
    pub fn metrics(&self) -> GlyphAtlasMetrics {
        GlyphAtlasMetrics {
            entries: self.entries.len(),
            ..self.metrics
        }
    }

    /// Maximum cached glyph entries.
    pub fn capacity(&self) -> usize {
        self.config.capacity()
    }

    fn touch(&mut self, key: GlyphKey) {
        self.lru.retain(|existing| *existing != key);
        self.lru.push_back(key);
    }

    fn evict_lru(&mut self) -> Result<GlyphSlot> {
        let key = self
            .lru
            .pop_front()
            .ok_or(GromaqError::GlyphAtlasInvariant {
                reason: "glyph atlas full with no LRU key",
            })?;
        let entry = self
            .entries
            .remove(&key)
            .ok_or(GromaqError::GlyphAtlasInvariant {
                reason: "glyph LRU key must exist in entries",
            })?;
        self.metrics.evictions += 1;
        Ok(GlyphSlot { entry })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                },
                GlyphVertex {
                    position: [1.0, 0.0],
                    uv: [1.0, 0.0],
                },
                GlyphVertex {
                    position: [1.0, 1.0],
                    uv: [1.0, 1.0],
                },
                GlyphVertex {
                    position: [0.0, 1.0],
                    uv: [0.0, 1.0],
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
    fn glyph_quad_base_index_accepts_last_representable_quad() {
        let last_valid_quad = usize::try_from(u32::MAX / 4).unwrap();

        assert_eq!(
            checked_glyph_quad_base_index(last_valid_quad).unwrap(),
            u32::MAX - 3
        );
    }

    #[test]
    fn glyph_quad_base_index_rejects_overflowing_quad_count() {
        let first_invalid_quad = usize::try_from(u32::MAX / 4).unwrap() + 1;

        let error = checked_glyph_quad_base_index(first_invalid_quad).unwrap_err();

        assert_eq!(error, GlyphQuadError::IndexCountTooLarge);
    }

    #[test]
    fn glyph_quad_index_capacity_uses_checked_multiplication() {
        assert_eq!(checked_glyph_quad_index_capacity(7).unwrap(), 42);

        let error = checked_glyph_quad_index_capacity((usize::MAX / 6) + 1).unwrap_err();

        assert_eq!(error, GlyphQuadError::IndexCountTooLarge);
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
    fn rgba_row_offset_uses_checked_multiplication() {
        assert_eq!(checked_rgba_row_offset(3, 8).unwrap(), 24);

        let error = checked_rgba_row_offset((usize::MAX / 8) + 1, 8).unwrap_err();

        assert_eq!(error, "rgba row offset is too large");
    }

    #[test]
    fn glyph_atlas_eviction_reports_missing_lru_key_invariant() {
        let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(1).unwrap());
        atlas.free_slots.clear();

        let error = atlas.evict_lru().unwrap_err();

        assert_eq!(
            error,
            GromaqError::GlyphAtlasInvariant {
                reason: "glyph atlas full with no LRU key",
            }
        );
    }

    #[test]
    fn glyph_atlas_eviction_reports_lru_entry_map_mismatch() {
        let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(1).unwrap());
        atlas.free_slots.clear();
        atlas
            .lru
            .push_back(GlyphKey::new('A', Style::default(), 14));

        let error = atlas.evict_lru().unwrap_err();

        assert_eq!(
            error,
            GromaqError::GlyphAtlasInvariant {
                reason: "glyph LRU key must exist in entries",
            }
        );
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
        scheduler.metrics.dropped_frames = u64::MAX - 1;
        let start = Instant::now();
        scheduler.record_presented(start);

        scheduler.record_presented(start + Duration::from_secs(4));

        assert_eq!(scheduler.metrics().dropped_frames, u64::MAX);
    }

    #[test]
    fn frame_scheduler_presented_frame_metrics_saturate() {
        let mut scheduler = FrameScheduler::new(144).unwrap();
        scheduler.metrics.frames_presented = u64::MAX;

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
            batch: &batch,
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
            batch: &batch,
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
    fn surface_glyph_vertex_byte_capacity_uses_checked_multiplication() {
        assert_eq!(surface_glyph_vertex_byte_capacity(2).unwrap(), 128);

        let error = surface_glyph_vertex_byte_capacity((usize::MAX / 64) + 1).unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame("surface glyph vertex bytes are too large".to_owned())
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
