//! Native `wgpu` device bootstrap.

mod bootstrap;
mod draw_buffers;
mod offscreen;
mod quad_bytes;
mod readback;
mod reports;
mod shaders;
mod smoke;
mod surface;
mod text_smoke;
mod upload;

pub use bootstrap::{
    GpuAdapterSnapshot, GpuBootstrap, GpuBootstrapBackend, GpuBootstrapConfig, GpuBootstrapError,
    GpuBootstrapRequest, GpuPowerPreference, NativeGpuContext, NativeWgpuBackend,
};
use offscreen::{GlyphDrawInput, draw_glyph_quads_rgba8};
pub use readback::ReadbackLayout;
pub use reports::{
    GpuGlyphAtlasUploadReport, GpuGlyphAtlasUploadRunner, GpuSmokeReport, GpuSmokeRunner,
    GpuTerminalTextPerfReport, GpuTerminalTextPerfRunner, GpuTerminalTextReport,
    GpuTerminalTextRunner, GpuTerminalTextSnapshotReport, GpuTerminalTextSnapshotRunner,
    GpuTextAtlasUploadReport, GpuTextAtlasUploadRunner, GpuTextureUploadReport,
    GpuTextureUploadRunner, GpuTexturedQuadReport, GpuTexturedQuadRunner,
};
pub use surface::{GpuSurfaceError, NativeGpuWindowSurface};
pub use upload::{UploadPattern, UploadPatternLayout};
