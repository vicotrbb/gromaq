use super::GpuBootstrapError;
use std::path::Path;

mod terminal_text;

pub use terminal_text::{
    GpuTerminalTextPerfReport, GpuTerminalTextPerfRunner, GpuTerminalTextReport,
    GpuTerminalTextRunner, GpuTerminalTextSnapshotReport, GpuTerminalTextSnapshotRunner,
};

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

/// Result of a welcome splash image snapshot rendered offscreen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuWelcomeImageSnapshotReport {
    /// Render target width in pixels.
    pub width: u32,
    /// Render target height in pixels.
    pub height: u32,
    /// PPM bytes written to the snapshot path.
    pub bytes_written: usize,
    /// Background corner pixel (expected to match the theme background).
    pub background_pixel: [u8; 4],
    /// A pixel sampled from the centered avatar region.
    pub image_pixel: [u8; 4],
    /// Number of pixels that differ from the background (the avatar coverage).
    pub drawn_pixels: usize,
}

/// Interface for contexts that can render the welcome avatar image offscreen.
pub trait GpuWelcomeImageSnapshotRunner {
    /// Render the welcome splash avatar image to a PPM snapshot.
    fn run_welcome_image_snapshot(
        &self,
        path: &Path,
    ) -> std::result::Result<GpuWelcomeImageSnapshotReport, GpuBootstrapError>;
}
