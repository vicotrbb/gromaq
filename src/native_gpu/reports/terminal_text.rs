use std::path::Path;

use super::super::GpuBootstrapError;

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
