//! Native `wgpu` device bootstrap.

use std::borrow::Cow;

use thiserror::Error;

mod draw_buffers;
mod quad_bytes;
mod readback;
mod reports;
mod shaders;
mod text_smoke;
mod upload;
use draw_buffers::{
    checked_textured_index_count, validate_background_draw_buffers, validate_textured_draw_buffers,
};
use quad_bytes::{
    background_quad_index_bytes, background_quad_vertex_bytes, glyph_quad_index_bytes,
    glyph_quad_vertex_bytes, textured_quad_index_bytes, textured_quad_vertex_bytes,
};
pub use readback::ReadbackLayout;
use readback::{last_rgba_pixel, read_texture_rgba8, rgba_pixel_at};
pub use reports::{
    GpuGlyphAtlasUploadReport, GpuGlyphAtlasUploadRunner, GpuSmokeReport, GpuSmokeRunner,
    GpuTerminalTextReport, GpuTerminalTextRunner, GpuTextAtlasUploadReport,
    GpuTextAtlasUploadRunner, GpuTextureUploadReport, GpuTextureUploadRunner,
    GpuTexturedQuadReport, GpuTexturedQuadRunner,
};
use shaders::{BACKGROUND_QUAD_WGSL, TEXTURED_QUAD_WGSL};
use text_smoke::{build_text_atlas_smoke_frame, build_text_atlas_smoke_image};
pub use upload::{UploadPattern, UploadPatternLayout};

use crate::renderer::{
    BackgroundQuadBatch, BackgroundQuadConfig, BackgroundQuadPlanner, CursorQuadConfig,
    CursorQuadPlanner, GlyphAtlasImage, GlyphQuadBatch, GlyphQuadConfig, GlyphQuadPlanner,
    TextDecorationQuadConfig, TextDecorationQuadPlanner, WgpuSurfaceBackend,
};

/// Power preference used when choosing a GPU adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuPowerPreference {
    /// Do not prefer a power class.
    None,
    /// Prefer lower power usage.
    LowPower,
    /// Prefer the highest performance adapter available.
    HighPerformance,
}

impl From<GpuPowerPreference> for wgpu::PowerPreference {
    fn from(value: GpuPowerPreference) -> Self {
        match value {
            GpuPowerPreference::None => Self::None,
            GpuPowerPreference::LowPower => Self::LowPower,
            GpuPowerPreference::HighPerformance => Self::HighPerformance,
        }
    }
}

/// Configuration for native GPU bootstrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuBootstrapConfig {
    /// Adapter power preference.
    pub power_preference: GpuPowerPreference,
    /// Whether software fallback adapters may be selected.
    pub force_fallback_adapter: bool,
    /// Debug label for the render device.
    pub device_label: &'static str,
}

impl GpuBootstrapConfig {
    /// Native defaults for the performance-first terminal renderer.
    pub fn native_default() -> Self {
        Self {
            power_preference: GpuPowerPreference::HighPerformance,
            force_fallback_adapter: false,
            device_label: "gromaq-render-device",
        }
    }
}

/// Fully resolved GPU bootstrap request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuBootstrapRequest {
    /// Adapter power preference.
    pub power_preference: GpuPowerPreference,
    /// Whether software fallback adapters may be selected.
    pub force_fallback_adapter: bool,
    /// Whether required feature set is empty.
    pub required_features_empty: bool,
    /// Debug label for the render device.
    pub device_label: &'static str,
}

/// Stable adapter metadata for diagnostics and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuAdapterSnapshot {
    /// Adapter name.
    pub name: String,
    /// Backend name.
    pub backend: String,
    /// Device type name.
    pub device_type: String,
    /// Backend-specific vendor ID.
    pub vendor: u32,
    /// Backend-specific device ID.
    pub device: u32,
}

/// Native GPU bootstrap errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GpuBootstrapError {
    /// No compatible adapter was available.
    #[error("no compatible GPU adapter available: {0}")]
    AdapterUnavailable(String),
    /// Device creation failed for the selected adapter.
    #[error("GPU device creation failed: {0}")]
    DeviceUnavailable(String),
    /// GPU smoke rendering or readback failed.
    #[error("GPU smoke readback failed: {0}")]
    SmokeReadback(String),
}

/// Errors produced while creating a native window-backed GPU surface.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GpuSurfaceError {
    /// Native `wgpu` surface creation failed.
    #[error("GPU surface creation failed: {0}")]
    CreateSurface(String),
}

/// Window surface backend plus capabilities ready for app-owned configuration.
#[derive(Debug)]
pub struct NativeGpuWindowSurface<B> {
    backend: B,
    capabilities: wgpu::SurfaceCapabilities,
}

impl<B> NativeGpuWindowSurface<B> {
    /// Create a native GPU window surface handoff object.
    pub fn new(backend: B, capabilities: wgpu::SurfaceCapabilities) -> Self {
        Self {
            backend,
            capabilities,
        }
    }

    /// Surface capabilities reported for the selected adapter and surface.
    pub fn capabilities(&self) -> &wgpu::SurfaceCapabilities {
        &self.capabilities
    }

    /// Consume the handoff object into backend and capabilities.
    pub fn into_parts(self) -> (B, wgpu::SurfaceCapabilities) {
        (self.backend, self.capabilities)
    }
}

/// Backend abstraction used to test bootstrap policy without requiring hardware.
pub trait GpuBootstrapBackend {
    /// Created context type.
    type Context;

    /// Request a GPU device for `request`.
    fn request_device(
        &self,
        request: &GpuBootstrapRequest,
    ) -> std::result::Result<Self::Context, GpuBootstrapError>;
}

/// Native GPU bootstrap coordinator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuBootstrap {
    config: GpuBootstrapConfig,
}

impl GpuBootstrap {
    /// Create a native GPU bootstrap coordinator.
    pub fn new(config: GpuBootstrapConfig) -> Self {
        Self { config }
    }

    /// Build the concrete request derived from configuration.
    pub fn request(&self) -> GpuBootstrapRequest {
        GpuBootstrapRequest {
            power_preference: self.config.power_preference,
            force_fallback_adapter: self.config.force_fallback_adapter,
            required_features_empty: true,
            device_label: self.config.device_label,
        }
    }

    /// Initialize a GPU context with a backend implementation.
    pub fn initialize_with<B: GpuBootstrapBackend>(
        &self,
        backend: &B,
    ) -> std::result::Result<B::Context, GpuBootstrapError> {
        backend.request_device(&self.request())
    }

    /// Initialize a real native `wgpu` context.
    pub fn initialize_native(&self) -> std::result::Result<NativeGpuContext, GpuBootstrapError> {
        self.initialize_with(&NativeWgpuBackend)
    }
}

/// Real `wgpu` backend for native GPU bootstrap.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeWgpuBackend;

impl GpuBootstrapBackend for NativeWgpuBackend {
    type Context = NativeGpuContext;

    fn request_device(
        &self,
        request: &GpuBootstrapRequest,
    ) -> std::result::Result<Self::Context, GpuBootstrapError> {
        pollster::block_on(request_native_wgpu_context(request))
    }
}

/// Live native GPU context.
#[derive(Debug)]
pub struct NativeGpuContext {
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: GpuAdapterSnapshot,
}

impl NativeGpuContext {
    /// Stable adapter metadata.
    pub fn adapter(&self) -> &GpuAdapterSnapshot {
        &self.adapter
    }

    /// Native `wgpu` device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Native `wgpu` queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Create a safe native window surface and collect adapter-specific capabilities.
    pub fn create_window_surface<'window>(
        &self,
        target: impl Into<wgpu::SurfaceTarget<'window>>,
    ) -> std::result::Result<NativeGpuWindowSurface<WgpuSurfaceBackend<'window>>, GpuSurfaceError>
    {
        let surface = self
            ._instance
            .create_surface(target)
            .map_err(|error| GpuSurfaceError::CreateSurface(error.to_string()))?;
        let capabilities = surface.get_capabilities(&self._adapter);
        let backend = WgpuSurfaceBackend::new(surface, &self.device, &self.queue);
        Ok(NativeGpuWindowSurface::new(backend, capabilities))
    }

    /// Clear an offscreen render target and read back its RGBA8 pixels.
    pub fn clear_offscreen_rgba8(
        &self,
        width: u32,
        height: u32,
        color: [f64; 4],
    ) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
        clear_offscreen_rgba8(&self.device, &self.queue, width, height, color)
    }

    /// Upload RGBA8 pixels into a texture and read them back.
    pub fn upload_rgba8_and_readback(
        &self,
        pattern: &UploadPattern,
    ) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
        upload_rgba8_and_readback(&self.device, &self.queue, pattern)
    }

    /// Upload a packed glyph atlas image into a texture and read it back.
    pub fn upload_glyph_atlas_and_readback(
        &self,
        image: &GlyphAtlasImage,
    ) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
        let pattern = UploadPattern::from_glyph_atlas_image(image);
        upload_rgba8_and_readback(&self.device, &self.queue, &pattern)
    }

    /// Draw a textured quad into an offscreen render target and read it back.
    pub fn draw_textured_quad_and_readback(
        &self,
        pattern: &UploadPattern,
        width: u32,
        height: u32,
    ) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
        draw_textured_quad_rgba8(&self.device, &self.queue, pattern, width, height)
    }
}

impl GpuSmokeRunner for NativeGpuContext {
    fn run_smoke(&self) -> std::result::Result<GpuSmokeReport, GpuBootstrapError> {
        let width = 4;
        let height = 4;
        let pixels = self.clear_offscreen_rgba8(width, height, [0.1, 0.2, 0.3, 1.0])?;
        let first_pixel = pixels
            .get(0..4)
            .ok_or_else(|| GpuBootstrapError::SmokeReadback("empty readback".to_owned()))?;
        Ok(GpuSmokeReport {
            width,
            height,
            first_pixel: [
                first_pixel[0],
                first_pixel[1],
                first_pixel[2],
                first_pixel[3],
            ],
            nonzero_bytes: pixels.iter().filter(|byte| **byte != 0).count(),
        })
    }
}

impl GpuTextureUploadRunner for NativeGpuContext {
    fn run_texture_upload_smoke(
        &self,
    ) -> std::result::Result<GpuTextureUploadReport, GpuBootstrapError> {
        let pattern = UploadPattern::checker_rgba8_2x2();
        let pixels = self.upload_rgba8_and_readback(&pattern)?;
        let first_pixel = pixels
            .get(0..4)
            .ok_or_else(|| GpuBootstrapError::SmokeReadback("empty upload readback".to_owned()))?;
        let last_pixel = last_rgba_pixel(&pixels, "upload readback")?;
        let matching_bytes = pixels
            .iter()
            .zip(pattern.rgba.iter())
            .filter(|(actual, expected)| actual == expected)
            .count();
        Ok(GpuTextureUploadReport {
            width: pattern.width,
            height: pattern.height,
            first_pixel: [
                first_pixel[0],
                first_pixel[1],
                first_pixel[2],
                first_pixel[3],
            ],
            last_pixel: [last_pixel[0], last_pixel[1], last_pixel[2], last_pixel[3]],
            matching_bytes,
            total_bytes: pattern.rgba.len(),
        })
    }
}

impl GpuGlyphAtlasUploadRunner for NativeGpuContext {
    fn run_glyph_atlas_upload_smoke(
        &self,
    ) -> std::result::Result<GpuGlyphAtlasUploadReport, GpuBootstrapError> {
        let image = GlyphAtlasImage::smoke_rgba8()
            .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
        let pixels = self.upload_glyph_atlas_and_readback(&image)?;
        let first_pixel = pixels.get(0..4).ok_or_else(|| {
            GpuBootstrapError::SmokeReadback("empty glyph atlas readback".to_owned())
        })?;
        let second_slot_first_pixel = rgba_pixel_at(&pixels, 2, "second glyph slot")?;
        let matching_bytes = pixels
            .iter()
            .zip(image.rgba.iter())
            .filter(|(actual, expected)| actual == expected)
            .count();
        Ok(GpuGlyphAtlasUploadReport {
            width: image.width,
            height: image.height,
            occupied_slots: image.occupied_slots,
            first_pixel: [
                first_pixel[0],
                first_pixel[1],
                first_pixel[2],
                first_pixel[3],
            ],
            second_slot_first_pixel: [
                second_slot_first_pixel[0],
                second_slot_first_pixel[1],
                second_slot_first_pixel[2],
                second_slot_first_pixel[3],
            ],
            matching_bytes,
            total_bytes: image.rgba.len(),
        })
    }
}

impl GpuTextAtlasUploadRunner for NativeGpuContext {
    fn run_text_atlas_upload_smoke(
        &self,
    ) -> std::result::Result<GpuTextAtlasUploadReport, GpuBootstrapError> {
        let (image, batch) = build_text_atlas_smoke_image()?;
        let pixels = self.upload_glyph_atlas_and_readback(&image)?;
        let matching_bytes = pixels
            .iter()
            .zip(image.rgba.iter())
            .filter(|(actual, expected)| actual == expected)
            .count();
        let covered_pixels = image
            .rgba
            .chunks_exact(4)
            .filter(|pixel| pixel[3] != 0)
            .count();
        Ok(GpuTextAtlasUploadReport {
            width: image.width,
            height: image.height,
            occupied_slots: image.occupied_slots,
            rasterized_glyphs: batch.rasterized,
            reused_glyphs: batch.reused,
            matching_bytes,
            total_bytes: image.rgba.len(),
            covered_pixels,
        })
    }
}

impl GpuTexturedQuadRunner for NativeGpuContext {
    fn run_textured_quad_smoke(
        &self,
    ) -> std::result::Result<GpuTexturedQuadReport, GpuBootstrapError> {
        let width = 4;
        let height = 4;
        let pixels = self.draw_textured_quad_and_readback(
            &UploadPattern::checker_rgba8_2x2(),
            width,
            height,
        )?;
        let first_pixel = pixels.get(0..4).ok_or_else(|| {
            GpuBootstrapError::SmokeReadback("empty textured quad readback".to_owned())
        })?;
        Ok(GpuTexturedQuadReport {
            width,
            height,
            first_pixel: [
                first_pixel[0],
                first_pixel[1],
                first_pixel[2],
                first_pixel[3],
            ],
            drawn_pixels: pixels.chunks_exact(4).filter(|pixel| pixel[3] != 0).count(),
        })
    }
}

impl GpuTerminalTextRunner for NativeGpuContext {
    fn run_terminal_text_smoke(
        &self,
    ) -> std::result::Result<GpuTerminalTextReport, GpuBootstrapError> {
        let frame = build_text_atlas_smoke_frame()?;
        let quad_config = GlyphQuadConfig {
            cell_width_px: frame.slot_width,
            cell_height_px: frame.slot_height,
            atlas_slot_width_px: frame.slot_width,
            atlas_slot_height_px: frame.slot_height,
            atlas_columns: frame.atlas_columns,
            atlas_width_px: frame.image.width,
            atlas_height_px: frame.image.height,
        };
        let quad_batch = GlyphQuadPlanner::new(quad_config)
            .plan(&frame.plan)
            .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
        let background_batch = BackgroundQuadPlanner::new(BackgroundQuadConfig {
            cell_width_px: frame.slot_width,
            cell_height_px: frame.slot_height,
        })
        .plan(&frame.plan)
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
        let decoration_batch = TextDecorationQuadPlanner::new(TextDecorationQuadConfig {
            cell_width_px: frame.slot_width,
            cell_height_px: frame.slot_height,
        })
        .plan(&frame.plan)
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
        let cursor_batch = CursorQuadPlanner::new(CursorQuadConfig {
            cell_width_px: frame.slot_width,
            cell_height_px: frame.slot_height,
            color_rgba8: [229, 229, 229, 255],
        })
        .plan(&frame.plan)
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
        let (target_width, target_height) = checked_terminal_text_target_dimensions(
            frame.plan.viewport_cols,
            frame.plan.viewport_rows,
            frame.slot_width,
            frame.slot_height,
        )?;
        let pixels = draw_glyph_quads_rgba8(
            &self.device,
            &self.queue,
            GlyphDrawInput {
                image: &frame.image,
                background_batch: &background_batch,
                batch: &quad_batch,
                decoration_batch: &decoration_batch,
                cursor_batch: &cursor_batch,
                width: target_width,
                height: target_height,
            },
        )?;
        Ok(GpuTerminalTextReport {
            width: target_width,
            height: target_height,
            glyphs: frame.plan.glyphs.len(),
            background_quads: background_batch.quads.len(),
            quads: quad_batch.quads.len(),
            decoration_quads: decoration_batch.quads.len(),
            cursor_quads: cursor_batch.quads.len(),
            rasterized_glyphs: frame.batch.rasterized,
            reused_glyphs: frame.batch.reused,
            first_drawn_pixel: first_nontransparent_pixel(&pixels),
            cursor_pixel: first_cursor_pixel(&cursor_batch, &pixels, target_width)?,
            drawn_pixels: pixels.chunks_exact(4).filter(|pixel| pixel[3] != 0).count(),
        })
    }
}

fn checked_terminal_text_target_dimensions(
    cols: u16,
    rows: u16,
    slot_width: u32,
    slot_height: u32,
) -> std::result::Result<(u32, u32), GpuBootstrapError> {
    let width = u32::from(cols).checked_mul(slot_width).ok_or_else(|| {
        GpuBootstrapError::SmokeReadback(
            "terminal text target width is too large to represent".to_owned(),
        )
    })?;
    let height = u32::from(rows).checked_mul(slot_height).ok_or_else(|| {
        GpuBootstrapError::SmokeReadback(
            "terminal text target height is too large to represent".to_owned(),
        )
    })?;
    Ok((width, height))
}

fn first_nontransparent_pixel(pixels: &[u8]) -> [u8; 4] {
    pixels
        .chunks_exact(4)
        .find(|pixel| pixel[3] != 0)
        .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
        .unwrap_or([0, 0, 0, 0])
}

fn first_cursor_pixel(
    cursor_batch: &BackgroundQuadBatch,
    pixels: &[u8],
    width: u32,
) -> std::result::Result<[u8; 4], GpuBootstrapError> {
    let Some(cursor) = cursor_batch.quads.first() else {
        return Ok([0, 0, 0, 0]);
    };
    let x = cursor.vertices[0].position[0] as u32;
    let y = cursor.vertices[0].position[1] as u32;
    let pixel_index =
        usize::try_from(u64::from(y) * u64::from(width) + u64::from(x)).map_err(|_| {
            GpuBootstrapError::SmokeReadback("cursor pixel offset is too large".to_owned())
        })?;
    let pixel = rgba_pixel_at(pixels, pixel_index, "cursor pixel")?;
    Ok([pixel[0], pixel[1], pixel[2], pixel[3]])
}

async fn request_native_wgpu_context(
    request: &GpuBootstrapRequest,
) -> std::result::Result<NativeGpuContext, GpuBootstrapError> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: request.power_preference.into(),
            force_fallback_adapter: request.force_fallback_adapter,
            compatible_surface: None,
        })
        .await
        .map_err(|error| GpuBootstrapError::AdapterUnavailable(error.to_string()))?;
    let info = adapter.get_info();
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some(request.device_label),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|error| GpuBootstrapError::DeviceUnavailable(error.to_string()))?;

    Ok(NativeGpuContext {
        _instance: instance,
        _adapter: adapter,
        device,
        queue,
        adapter: GpuAdapterSnapshot {
            name: info.name,
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
            vendor: info.vendor,
            device: info.device,
        },
    })
}

fn clear_offscreen_rgba8(
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

fn upload_rgba8_and_readback(
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

fn draw_textured_quad_rgba8(
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

fn draw_glyph_quads_rgba8(
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

struct GlyphDrawInput<'a> {
    image: &'a GlyphAtlasImage,
    background_batch: &'a BackgroundQuadBatch,
    batch: &'a GlyphQuadBatch,
    decoration_batch: &'a BackgroundQuadBatch,
    cursor_batch: &'a BackgroundQuadBatch,
    width: u32,
    height: u32,
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
) -> std::result::Result<draw_buffers::DrawBufferLayout, GpuBootstrapError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_text_target_dimensions_reports_checked_size() {
        let dimensions = checked_terminal_text_target_dimensions(80, 24, 8, 16).unwrap();

        assert_eq!(dimensions, (640, 384));
    }

    #[test]
    fn terminal_text_target_dimensions_rejects_overflowing_width() {
        let error = checked_terminal_text_target_dimensions(2, 1, u32::MAX, 1).unwrap_err();

        assert_eq!(
            error,
            GpuBootstrapError::SmokeReadback(
                "terminal text target width is too large to represent".to_owned()
            )
        );
    }

    #[test]
    fn terminal_text_target_dimensions_rejects_overflowing_height() {
        let error = checked_terminal_text_target_dimensions(1, 2, 1, u32::MAX).unwrap_err();

        assert_eq!(
            error,
            GpuBootstrapError::SmokeReadback(
                "terminal text target height is too large to represent".to_owned()
            )
        );
    }
}
