//! GPU CLI context abstraction and metadata-only fallback adapters.

use std::path::Path;

use crate::native_gpu::{
    GpuAdapterSnapshot, GpuBootstrapError, GpuGlyphAtlasUploadRunner, GpuSmokeRunner,
    GpuTerminalTextPerfRunner, GpuTerminalTextRunner, GpuTerminalTextSnapshotRunner,
    GpuTextAtlasUploadRunner, GpuTextureUploadRunner, GpuTexturedQuadRunner,
};

/// Adapter metadata reporting abstraction.
pub trait AdapterReport {
    /// Return stable adapter metadata.
    fn adapter_report(&self) -> &GpuAdapterSnapshot;
}

/// Full GPU context capability set required by CLI GPU commands.
pub trait GpuCommandContext:
    AdapterReport
    + GpuSmokeRunner
    + GpuTextureUploadRunner
    + GpuGlyphAtlasUploadRunner
    + GpuTextAtlasUploadRunner
    + GpuTexturedQuadRunner
    + GpuTerminalTextRunner
    + GpuTerminalTextPerfRunner
    + GpuTerminalTextSnapshotRunner
{
}

impl<T> GpuCommandContext for T where
    T: AdapterReport
        + GpuSmokeRunner
        + GpuTextureUploadRunner
        + GpuGlyphAtlasUploadRunner
        + GpuTextAtlasUploadRunner
        + GpuTexturedQuadRunner
        + GpuTerminalTextRunner
        + GpuTerminalTextPerfRunner
        + GpuTerminalTextSnapshotRunner
{
}

impl AdapterReport for GpuAdapterSnapshot {
    fn adapter_report(&self) -> &GpuAdapterSnapshot {
        self
    }
}

impl GpuSmokeRunner for GpuAdapterSnapshot {
    fn run_smoke(&self) -> Result<crate::native_gpu::GpuSmokeReport, GpuBootstrapError> {
        Err(metadata_without_live_context_error())
    }
}

impl GpuTextureUploadRunner for GpuAdapterSnapshot {
    fn run_texture_upload_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuTextureUploadReport, GpuBootstrapError> {
        Err(metadata_without_live_context_error())
    }
}

impl GpuGlyphAtlasUploadRunner for GpuAdapterSnapshot {
    fn run_glyph_atlas_upload_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuGlyphAtlasUploadReport, GpuBootstrapError> {
        Err(metadata_without_live_context_error())
    }
}

impl GpuTextAtlasUploadRunner for GpuAdapterSnapshot {
    fn run_text_atlas_upload_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuTextAtlasUploadReport, GpuBootstrapError> {
        Err(metadata_without_live_context_error())
    }
}

impl GpuTexturedQuadRunner for GpuAdapterSnapshot {
    fn run_textured_quad_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuTexturedQuadReport, GpuBootstrapError> {
        Err(metadata_without_live_context_error())
    }
}

impl GpuTerminalTextRunner for GpuAdapterSnapshot {
    fn run_terminal_text_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuTerminalTextReport, GpuBootstrapError> {
        Err(metadata_without_live_context_error())
    }
}

impl GpuTerminalTextPerfRunner for GpuAdapterSnapshot {
    fn run_terminal_text_perf_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuTerminalTextPerfReport, GpuBootstrapError> {
        Err(metadata_without_live_context_error())
    }
}

impl GpuTerminalTextSnapshotRunner for GpuAdapterSnapshot {
    fn run_terminal_text_snapshot(
        &self,
        _path: &Path,
    ) -> Result<crate::native_gpu::GpuTerminalTextSnapshotReport, GpuBootstrapError> {
        Err(metadata_without_live_context_error())
    }
}

impl AdapterReport for crate::native_gpu::NativeGpuContext {
    fn adapter_report(&self) -> &GpuAdapterSnapshot {
        self.adapter()
    }
}

fn metadata_without_live_context_error() -> GpuBootstrapError {
    GpuBootstrapError::SmokeReadback("adapter metadata does not own a live GPU context".to_owned())
}
