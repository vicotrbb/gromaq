//! Native `wgpu` device bootstrap.

use thiserror::Error;

mod draw_buffers;
mod offscreen;
mod quad_bytes;
mod readback;
mod reports;
mod shaders;
mod smoke;
mod text_smoke;
mod upload;
use offscreen::{
    GlyphDrawInput, clear_offscreen_rgba8, draw_glyph_quads_rgba8, draw_textured_quad_rgba8,
    upload_rgba8_and_readback,
};
pub use readback::ReadbackLayout;
pub use reports::{
    GpuGlyphAtlasUploadReport, GpuGlyphAtlasUploadRunner, GpuSmokeReport, GpuSmokeRunner,
    GpuTerminalTextReport, GpuTerminalTextRunner, GpuTextAtlasUploadReport,
    GpuTextAtlasUploadRunner, GpuTextureUploadReport, GpuTextureUploadRunner,
    GpuTexturedQuadReport, GpuTexturedQuadRunner,
};
pub use upload::{UploadPattern, UploadPatternLayout};

use crate::renderer::{GlyphAtlasImage, WgpuSurfaceBackend};

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
