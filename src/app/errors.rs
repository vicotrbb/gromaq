use thiserror::Error;
use winit::error::{EventLoopError, OsError};

use crate::error::GromaqError;
use crate::font::FontRasterError;
use crate::native_gpu::{GpuBootstrapError, GpuSurfaceError};
use crate::renderer::{SurfaceConfigError, SurfaceFrameError};

/// Errors from launching the native application loop.
#[derive(Debug, Error)]
pub enum NativeAppError {
    /// The event loop could not be created or executed.
    #[error("native event loop failed: {0}")]
    EventLoop(#[from] EventLoopError),
    /// The native window could not be created.
    #[error("native window creation failed: {0}")]
    WindowCreation(String),
    /// Native terminal runtime setup failed.
    #[error("native runtime failed: {0}")]
    Runtime(String),
    /// Native GPU setup failed.
    #[error("native GPU setup failed: {0}")]
    Gpu(String),
}

/// Errors while preparing or presenting a terminal glyph frame.
#[derive(Debug, Error)]
pub enum NativeGlyphFrameError {
    /// Font rasterization failed while building the glyph atlas image.
    #[error("native glyph rasterization failed: {0}")]
    Font(#[from] FontRasterError),
    /// Surface frame acquisition, drawing, or presentation failed.
    #[error("native glyph surface presentation failed: {0}")]
    Surface(#[from] SurfaceFrameError),
    /// CPU-side render planning failed before presentation.
    #[error("native glyph render planning failed: {0}")]
    Renderer(#[from] GromaqError),
}

impl From<OsError> for NativeAppError {
    fn from(value: OsError) -> Self {
        Self::WindowCreation(value.to_string())
    }
}

impl From<GromaqError> for NativeAppError {
    fn from(value: GromaqError) -> Self {
        Self::Runtime(value.to_string())
    }
}

impl From<GpuBootstrapError> for NativeAppError {
    fn from(value: GpuBootstrapError) -> Self {
        Self::Gpu(value.to_string())
    }
}

impl From<GpuSurfaceError> for NativeAppError {
    fn from(value: GpuSurfaceError) -> Self {
        Self::Gpu(value.to_string())
    }
}

impl From<SurfaceConfigError> for NativeAppError {
    fn from(value: SurfaceConfigError) -> Self {
        Self::Gpu(value.to_string())
    }
}

impl From<SurfaceFrameError> for NativeAppError {
    fn from(value: SurfaceFrameError) -> Self {
        Self::Gpu(value.to_string())
    }
}

impl From<FontRasterError> for NativeAppError {
    fn from(value: FontRasterError) -> Self {
        Self::Runtime(value.to_string())
    }
}

impl From<NativeGlyphFrameError> for NativeAppError {
    fn from(value: NativeGlyphFrameError) -> Self {
        match value {
            NativeGlyphFrameError::Font(error) => Self::Runtime(error.to_string()),
            NativeGlyphFrameError::Surface(error) => Self::Gpu(error.to_string()),
            NativeGlyphFrameError::Renderer(error) => Self::Runtime(error.to_string()),
        }
    }
}
