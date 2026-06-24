use super::GpuBootstrapError;
use std::path::Path;

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

/// Result of a live GPU draw/readback using terminal-planned real-font glyphs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuTerminalTextReport {
    /// Render target width in pixels.
    pub width: u32,
    /// Render target height in pixels.
    pub height: u32,
    /// Number of terminal glyph draw commands in the render plan.
    pub glyphs: usize,
    /// Number of solid background quads drawn before glyph quads.
    pub background_quads: usize,
    /// Number of textured glyph quads drawn.
    pub quads: usize,
    /// Number of solid text-decoration quads drawn after glyph quads.
    pub decoration_quads: usize,
    /// Number of solid cursor quads drawn after glyph quads.
    pub cursor_quads: usize,
    /// Count of distinct glyphs rasterized from the font.
    pub rasterized_glyphs: usize,
    /// Count of planned glyphs reused from the rasterized glyph cache.
    pub reused_glyphs: usize,
    /// First non-transparent RGBA8 output pixel after drawing.
    pub first_drawn_pixel: [u8; 4],
    /// Sampled terminal background pixel from the themed text surface.
    pub background_pixel: [u8; 4],
    /// Sampled foreground glyph pixel distinct from background and cursor pixels.
    pub glyph_pixel: [u8; 4],
    /// WCAG contrast ratio between sampled foreground glyph and background, multiplied by 100.
    pub glyph_background_contrast_x100: u32,
    /// First sampled RGBA8 pixel from the cursor quad after drawing.
    pub cursor_pixel: [u8; 4],
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

/// Result of exporting a live GPU terminal-text draw/readback to an image artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuTerminalTextSnapshotReport {
    /// Render target width in pixels.
    pub width: u32,
    /// Render target height in pixels.
    pub height: u32,
    /// Number of bytes written to the snapshot artifact.
    pub bytes_written: usize,
    /// Number of terminal glyph draw commands in the render plan.
    pub glyphs: usize,
    /// Sampled terminal background pixel from the themed text surface.
    pub background_pixel: [u8; 4],
    /// Sampled foreground glyph pixel distinct from background and cursor pixels.
    pub glyph_pixel: [u8; 4],
    /// WCAG contrast ratio between sampled foreground glyph and background, multiplied by 100.
    pub glyph_background_contrast_x100: u32,
    /// First sampled RGBA8 pixel from the cursor quad after drawing.
    pub cursor_pixel: [u8; 4],
    /// Number of output pixels with non-zero alpha after drawing.
    pub drawn_pixels: usize,
}

/// Interface for contexts that can export terminal text through the GPU pipeline.
pub trait GpuTerminalTextSnapshotRunner {
    /// Run a GPU terminal-text draw/readback and write an inspectable image artifact.
    fn run_terminal_text_snapshot(
        &self,
        path: &Path,
    ) -> std::result::Result<GpuTerminalTextSnapshotReport, GpuBootstrapError>;
}

/// Repeated live GPU terminal-text draw/readback timing summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuTerminalTextPerfReport {
    /// Number of draw/readback frames measured.
    pub frames: usize,
    /// Render target width in pixels.
    pub width: u32,
    /// Render target height in pixels.
    pub height: u32,
    /// Number of output pixels with non-zero alpha on the final measured frame.
    pub drawn_pixels: usize,
    /// Fastest measured draw/readback duration in nanoseconds.
    pub min_ns: u128,
    /// Average measured draw/readback duration in nanoseconds.
    pub avg_ns: u128,
    /// Slowest measured draw/readback duration in nanoseconds.
    pub max_ns: u128,
    /// Inclusive p95 draw/readback duration in nanoseconds.
    pub p95_ns: u128,
}

/// Interface for contexts that can time repeated terminal-text GPU draw/readbacks.
pub trait GpuTerminalTextPerfRunner {
    /// Run repeated terminal-text GPU draw/readback timing.
    fn run_terminal_text_perf_smoke(
        &self,
    ) -> std::result::Result<GpuTerminalTextPerfReport, GpuBootstrapError>;
}
