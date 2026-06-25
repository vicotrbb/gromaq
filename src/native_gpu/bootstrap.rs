//! Native GPU bootstrap policy and live context.

use thiserror::Error;

use super::offscreen::{
    clear_offscreen_rgba8, draw_textured_quad_rgba8, upload_rgba8_and_readback,
};
use super::surface::{GpuSurfaceError, NativeGpuWindowSurface};
use super::upload::UploadPattern;
use crate::renderer::{GlyphAtlasImage, WgpuSurfaceBackend};

mod config;
mod snapshot;

pub use config::{GpuBootstrapConfig, GpuBootstrapRequest, GpuPowerPreference};
pub use snapshot::GpuAdapterSnapshot;

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
