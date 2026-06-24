use thiserror::Error;

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
