//! Native `wgpu` device bootstrap.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use thiserror::Error;

use crate::font::{RasterizedGlyphBatch, RasterizedGlyphCache};
use crate::renderer::{
    GlyphAtlas, GlyphAtlasConfig, GlyphAtlasImage, GlyphQuadBatch, GlyphQuadConfig,
    GlyphQuadPlanner, RenderPlan, RenderPlanner, WgpuSurfaceBackend,
};
use crate::{Terminal, TerminalConfig};

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
        let last_pixel = pixels
            .get(pixels.len().saturating_sub(4)..)
            .ok_or_else(|| GpuBootstrapError::SmokeReadback("empty upload readback".to_owned()))?;
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
        let image = GlyphAtlasImage::smoke_rgba8();
        let pixels = self.upload_glyph_atlas_and_readback(&image)?;
        let first_pixel = pixels.get(0..4).ok_or_else(|| {
            GpuBootstrapError::SmokeReadback("empty glyph atlas readback".to_owned())
        })?;
        let second_slot_offset = usize::try_from(2 * 4).unwrap_or(usize::MAX);
        let second_slot_first_pixel = pixels
            .get(second_slot_offset..second_slot_offset + 4)
            .ok_or_else(|| {
                GpuBootstrapError::SmokeReadback("missing second glyph slot".to_owned())
            })?;
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
        let target_width = u32::from(frame.plan.viewport_cols).saturating_mul(frame.slot_width);
        let target_height = u32::from(frame.plan.viewport_rows).saturating_mul(frame.slot_height);
        let pixels = draw_glyph_quads_rgba8(
            &self.device,
            &self.queue,
            &frame.image,
            &quad_batch,
            target_width,
            target_height,
        )?;
        Ok(GpuTerminalTextReport {
            width: target_width,
            height: target_height,
            glyphs: frame.plan.glyphs.len(),
            quads: quad_batch.quads.len(),
            rasterized_glyphs: frame.batch.rasterized,
            reused_glyphs: frame.batch.reused,
            drawn_pixels: pixels.chunks_exact(4).filter(|pixel| pixel[3] != 0).count(),
        })
    }
}

/// Padded RGBA8 texture readback layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadbackLayout {
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
    /// Dense bytes per row without GPU copy padding.
    pub dense_bytes_per_row: u32,
    /// Padded bytes per row required by `wgpu` texture-to-buffer copies.
    pub padded_bytes_per_row: u32,
    /// Total readback buffer size.
    pub buffer_size: u64,
}

impl ReadbackLayout {
    /// Build a padded layout for an RGBA8 texture.
    pub fn rgba8(width: u32, height: u32) -> std::result::Result<Self, GpuBootstrapError> {
        let dense_bytes_per_row = width.checked_mul(4).ok_or_else(|| {
            GpuBootstrapError::SmokeReadback("RGBA8 row byte size is too large".to_owned())
        })?;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = dense_bytes_per_row
            .div_ceil(align)
            .checked_mul(align)
            .ok_or_else(|| {
                GpuBootstrapError::SmokeReadback(
                    "padded readback row byte size is too large".to_owned(),
                )
            })?;
        Ok(Self {
            width,
            height,
            dense_bytes_per_row,
            padded_bytes_per_row,
            buffer_size: u64::from(padded_bytes_per_row) * u64::from(height),
        })
    }
}

/// Result of a live GPU smoke render/readback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuSmokeReport {
    /// Render target width in pixels.
    pub width: u32,
    /// Render target height in pixels.
    pub height: u32,
    /// First RGBA8 pixel read from the GPU result.
    pub first_pixel: [u8; 4],
    /// Number of non-zero bytes in the dense readback.
    pub nonzero_bytes: usize,
}

/// Interface for contexts that can execute a GPU smoke render/readback.
pub trait GpuSmokeRunner {
    /// Run a GPU smoke render/readback.
    fn run_smoke(&self) -> std::result::Result<GpuSmokeReport, GpuBootstrapError>;
}

/// Deterministic RGBA8 upload pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadPattern {
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
    /// Dense RGBA8 pixels in row-major order.
    pub rgba: Vec<u8>,
}

/// Checked dense RGBA8 upload layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadPatternLayout {
    /// Dense bytes per row.
    pub row_bytes: u32,
    /// Expected total dense RGBA8 byte length.
    pub expected_len: usize,
}

impl UploadPattern {
    /// Build a 2x2 RGBA checker pattern.
    pub fn checker_rgba8_2x2() -> Self {
        Self {
            width: 2,
            height: 2,
            rgba: vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        }
    }

    /// Build an upload pattern from a packed glyph atlas image.
    pub fn from_glyph_atlas_image(image: &GlyphAtlasImage) -> Self {
        Self {
            width: image.width,
            height: image.height,
            rgba: image.rgba.clone(),
        }
    }

    /// Validate dimensions and return the dense RGBA8 upload layout.
    pub fn rgba8_layout(&self) -> std::result::Result<UploadPatternLayout, GpuBootstrapError> {
        if self.width == 0 || self.height == 0 {
            return Err(GpuBootstrapError::SmokeReadback(
                "upload pattern dimensions must be non-zero".to_owned(),
            ));
        }
        let row_bytes = self.width.checked_mul(4).ok_or_else(|| {
            GpuBootstrapError::SmokeReadback("upload pattern row byte size is too large".to_owned())
        })?;
        let expected_len = usize::try_from(row_bytes)
            .ok()
            .and_then(|row_bytes| {
                usize::try_from(self.height)
                    .ok()
                    .and_then(|height| row_bytes.checked_mul(height))
            })
            .ok_or_else(|| {
                GpuBootstrapError::SmokeReadback("upload pattern byte size is too large".to_owned())
            })?;
        Ok(UploadPatternLayout {
            row_bytes,
            expected_len,
        })
    }
}

/// Result of a live GPU texture upload/readback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuTextureUploadReport {
    /// Uploaded texture width in pixels.
    pub width: u32,
    /// Uploaded texture height in pixels.
    pub height: u32,
    /// First RGBA8 pixel read from the GPU result.
    pub first_pixel: [u8; 4],
    /// Last RGBA8 pixel read from the GPU result.
    pub last_pixel: [u8; 4],
    /// Number of bytes matching the source upload.
    pub matching_bytes: usize,
    /// Total uploaded bytes.
    pub total_bytes: usize,
}

/// Interface for contexts that can execute a GPU texture upload/readback smoke.
pub trait GpuTextureUploadRunner {
    /// Run a GPU texture upload/readback smoke test.
    fn run_texture_upload_smoke(
        &self,
    ) -> std::result::Result<GpuTextureUploadReport, GpuBootstrapError>;
}

/// Result of a live GPU glyph atlas upload/readback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuGlyphAtlasUploadReport {
    /// Uploaded atlas width in pixels.
    pub width: u32,
    /// Uploaded atlas height in pixels.
    pub height: u32,
    /// Number of occupied glyph atlas slots.
    pub occupied_slots: usize,
    /// First RGBA8 pixel read from the atlas.
    pub first_pixel: [u8; 4],
    /// First RGBA8 pixel of the second atlas slot.
    pub second_slot_first_pixel: [u8; 4],
    /// Number of bytes matching the source atlas image.
    pub matching_bytes: usize,
    /// Total uploaded atlas bytes.
    pub total_bytes: usize,
}

/// Interface for contexts that can execute a GPU glyph atlas upload/readback smoke.
pub trait GpuGlyphAtlasUploadRunner {
    /// Run a GPU glyph atlas upload/readback smoke test.
    fn run_glyph_atlas_upload_smoke(
        &self,
    ) -> std::result::Result<GpuGlyphAtlasUploadReport, GpuBootstrapError>;
}

/// Result of a live GPU upload/readback using real font-rasterized terminal glyphs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuTextAtlasUploadReport {
    /// Uploaded atlas width in pixels.
    pub width: u32,
    /// Uploaded atlas height in pixels.
    pub height: u32,
    /// Number of occupied glyph atlas slots.
    pub occupied_slots: usize,
    /// Count of glyphs rasterized from the font for this smoke frame.
    pub rasterized_glyphs: usize,
    /// Count of planned glyphs reused from the rasterized glyph cache.
    pub reused_glyphs: usize,
    /// Number of bytes matching the source atlas image.
    pub matching_bytes: usize,
    /// Total uploaded atlas bytes.
    pub total_bytes: usize,
    /// Number of atlas pixels with non-zero alpha coverage.
    pub covered_pixels: usize,
}

/// Interface for contexts that can upload/read back a real font-rasterized text atlas.
pub trait GpuTextAtlasUploadRunner {
    /// Run a GPU upload/readback smoke using font-backed terminal glyph atlas data.
    fn run_text_atlas_upload_smoke(
        &self,
    ) -> std::result::Result<GpuTextAtlasUploadReport, GpuBootstrapError>;
}

/// Result of a live GPU textured-quad draw/readback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuTexturedQuadReport {
    /// Render target width in pixels.
    pub width: u32,
    /// Render target height in pixels.
    pub height: u32,
    /// First RGBA8 pixel read from the rendered target.
    pub first_pixel: [u8; 4],
    /// Number of pixels with non-zero alpha after drawing.
    pub drawn_pixels: usize,
}

/// Interface for contexts that can draw a textured quad into a GPU render target.
pub trait GpuTexturedQuadRunner {
    /// Run a GPU textured-quad draw/readback smoke test.
    fn run_textured_quad_smoke(
        &self,
    ) -> std::result::Result<GpuTexturedQuadReport, GpuBootstrapError>;
}

/// Result of a live GPU draw/readback using terminal-planned real-font glyphs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuTerminalTextReport {
    /// Render target width in pixels.
    pub width: u32,
    /// Render target height in pixels.
    pub height: u32,
    /// Number of terminal glyph draw commands in the render plan.
    pub glyphs: usize,
    /// Number of textured glyph quads drawn.
    pub quads: usize,
    /// Count of distinct glyphs rasterized from the font.
    pub rasterized_glyphs: usize,
    /// Count of planned glyphs reused from the rasterized glyph cache.
    pub reused_glyphs: usize,
    /// Number of output pixels with non-zero alpha after drawing.
    pub drawn_pixels: usize,
}

/// Interface for contexts that can draw terminal text through the GPU pipeline.
pub trait GpuTerminalTextRunner {
    /// Run a GPU draw/readback smoke using terminal render-plan text.
    fn run_terminal_text_smoke(
        &self,
    ) -> std::result::Result<GpuTerminalTextReport, GpuBootstrapError>;
}

#[derive(Debug)]
struct TextAtlasSmokeFrame {
    image: GlyphAtlasImage,
    batch: RasterizedGlyphBatch,
    plan: RenderPlan,
    slot_width: u32,
    slot_height: u32,
    atlas_columns: u32,
}

fn build_text_atlas_smoke_image()
-> std::result::Result<(GlyphAtlasImage, RasterizedGlyphBatch), GpuBootstrapError> {
    let frame = build_text_atlas_smoke_frame()?;
    Ok((frame.image, frame.batch))
}

fn build_text_atlas_smoke_frame() -> std::result::Result<TextAtlasSmokeFrame, GpuBootstrapError> {
    let mut terminal = Terminal::new(
        TerminalConfig::new(8, 2)
            .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?,
    );
    terminal
        .write_str("A😀A")
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(
        GlyphAtlasConfig::new(8)
            .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?,
    );
    let mut planner = RenderPlanner::new(18);
    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
    let mut cache = RasterizedGlyphCache::from_font_bytes(system_smoke_font_bytes()?)
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
    let batch = cache
        .rasterize_plan(&plan)
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
    let slot_width = batch
        .bitmaps
        .iter()
        .map(|glyph| glyph.width)
        .max()
        .ok_or_else(|| GpuBootstrapError::SmokeReadback("empty text atlas batch".to_owned()))?;
    let slot_height = batch
        .bitmaps
        .iter()
        .map(|glyph| glyph.height)
        .max()
        .ok_or_else(|| GpuBootstrapError::SmokeReadback("empty text atlas batch".to_owned()))?;
    let padded = batch
        .bitmaps
        .iter()
        .map(|glyph| {
            glyph
                .padded_to(slot_width, slot_height)
                .map_err(GpuBootstrapError::SmokeReadback)
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let image = GlyphAtlasImage::pack_rgba8(slot_width, slot_height, 2, &padded)
        .map_err(GpuBootstrapError::SmokeReadback)?;
    Ok(TextAtlasSmokeFrame {
        image,
        batch,
        plan,
        slot_width,
        slot_height,
        atlas_columns: 2,
    })
}

fn system_mono_font_path() -> std::result::Result<PathBuf, GpuBootstrapError> {
    [
        "/System/Library/Fonts/SFNSMono.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
    ]
    .into_iter()
    .map(Path::new)
    .find(|path| path.exists())
    .map(Path::to_path_buf)
    .ok_or_else(|| {
        GpuBootstrapError::SmokeReadback(
            "no supported system monospace font found for text atlas smoke".to_owned(),
        )
    })
}

fn system_smoke_font_bytes() -> std::result::Result<Vec<Vec<u8>>, GpuBootstrapError> {
    let mut font_bytes = vec![
        std::fs::read(system_mono_font_path()?)
            .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?,
    ];
    for fallback_path in [
        "/System/Library/Fonts/Apple Color Emoji.ttc",
        "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
    ] {
        let path = Path::new(fallback_path);
        if path.exists() {
            font_bytes.push(
                std::fs::read(path)
                    .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?,
            );
        }
    }
    Ok(font_bytes)
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
    image: &GlyphAtlasImage,
    batch: &GlyphQuadBatch,
    width: u32,
    height: u32,
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    if batch.quads.is_empty() {
        return Err(GpuBootstrapError::SmokeReadback(
            "terminal text draw batch is empty".to_owned(),
        ));
    }
    let pattern = UploadPattern::from_glyph_atlas_image(image);
    let source_layout = validate_textured_source_pattern(&pattern)?;
    let vertices = glyph_quad_vertex_bytes(batch, width, height)?;
    let indices = glyph_quad_index_bytes(batch);
    draw_textured_vertices_rgba8(
        device,
        queue,
        TexturedDrawInput {
            pattern: &pattern,
            source_layout,
            vertex_bytes: &vertices,
            index_bytes: &indices,
            index_count: u32::try_from(batch.indices.len()).unwrap_or(u32::MAX),
            index_format: wgpu::IndexFormat::Uint32,
            width,
            height,
        },
    )
}

struct TexturedDrawInput<'a> {
    pattern: &'a UploadPattern,
    source_layout: UploadPatternLayout,
    vertex_bytes: &'a [u8],
    index_bytes: &'a [u8],
    index_count: u32,
    index_format: wgpu::IndexFormat,
    width: u32,
    height: u32,
}

fn draw_textured_vertices_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    input: TexturedDrawInput<'_>,
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    if input.vertex_bytes.is_empty() || input.index_bytes.is_empty() || input.index_count == 0 {
        return Err(GpuBootstrapError::SmokeReadback(
            "textured draw buffers must be non-empty".to_owned(),
        ));
    }

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
        label: Some("gromaq-textured-quad-vertices"),
        size: u64::try_from(input.vertex_bytes.len()).unwrap_or(u64::MAX),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gromaq-textured-quad-indices"),
        size: u64::try_from(input.index_bytes.len()).unwrap_or(u64::MAX),
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
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), input.index_format);
        pass.draw_indexed(0..input.index_count, 0, 0..1);
    }
    queue.submit([encoder.finish()]);
    read_texture_rgba8(device, queue, &target, input.width, input.height)
}

fn textured_quad_vertex_bytes() -> Vec<u8> {
    [
        [-1.0_f32, 1.0, 0.0, 0.0],
        [1.0, 1.0, 1.0, 0.0],
        [1.0, -1.0, 1.0, 1.0],
        [-1.0, -1.0, 0.0, 1.0],
    ]
    .into_iter()
    .flat_map(|vertex| vertex.into_iter().flat_map(f32::to_le_bytes))
    .collect()
}

fn textured_quad_index_bytes() -> Vec<u8> {
    [0_u16, 1, 2, 0, 2, 3]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn glyph_quad_vertex_bytes(
    batch: &GlyphQuadBatch,
    width: u32,
    height: u32,
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    if width == 0 || height == 0 {
        return Err(GpuBootstrapError::SmokeReadback(
            "terminal text render target dimensions must be non-zero".to_owned(),
        ));
    }
    let width = width as f32;
    let height = height as f32;
    let mut bytes = Vec::with_capacity(batch.quads.len().saturating_mul(4 * 4 * 4));
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

fn glyph_quad_index_bytes(batch: &GlyphQuadBatch) -> Vec<u8> {
    batch
        .indices
        .iter()
        .flat_map(|index| index.to_le_bytes())
        .collect()
}

const TEXTURED_QUAD_WGSL: &str = r#"
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

fn read_texture_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    let layout = ReadbackLayout::rgba8(width, height)?;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gromaq-texture-readback"),
        size: layout.buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gromaq-texture-readback-encoder"),
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(layout.padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    read_dense_rgba8_from_buffer(device, &readback, layout)
}

fn read_dense_rgba8_from_buffer(
    device: &wgpu::Device,
    readback: &wgpu::Buffer,
    layout: ReadbackLayout,
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    let slice = readback.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result.map_err(|error| error.to_string()));
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
    receiver
        .recv()
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?
        .map_err(GpuBootstrapError::SmokeReadback)?;

    let mapped = slice.get_mapped_range();
    let dense_len = usize::try_from(layout.dense_bytes_per_row)
        .ok()
        .and_then(|row_bytes| {
            usize::try_from(layout.height)
                .ok()
                .and_then(|height| row_bytes.checked_mul(height))
        })
        .ok_or_else(|| {
            GpuBootstrapError::SmokeReadback("dense readback size is too large".to_owned())
        })?;
    let mut dense = Vec::new();
    dense.try_reserve_exact(dense_len).map_err(|_| {
        GpuBootstrapError::SmokeReadback("dense readback buffer is too large".to_owned())
    })?;
    for row in 0..layout.height {
        let start = usize::try_from(u64::from(row) * u64::from(layout.padded_bytes_per_row))
            .map_err(|_| {
                GpuBootstrapError::SmokeReadback("readback row offset is too large".to_owned())
            })?;
        let row_bytes = usize::try_from(layout.dense_bytes_per_row).map_err(|_| {
            GpuBootstrapError::SmokeReadback("readback row byte size is too large".to_owned())
        })?;
        let end = start.checked_add(row_bytes).ok_or_else(|| {
            GpuBootstrapError::SmokeReadback("readback row end offset is too large".to_owned())
        })?;
        if end > mapped.len() {
            return Err(GpuBootstrapError::SmokeReadback(
                "readback row exceeds mapped buffer".to_owned(),
            ));
        }
        dense.extend_from_slice(&mapped[start..end]);
    }
    drop(mapped);
    readback.unmap();
    Ok(dense)
}
