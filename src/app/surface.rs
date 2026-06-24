use std::path::Path;

use crate::font::RasterizedGlyphCache;
use crate::native_gpu::NativeGpuWindowSurface;
use crate::renderer::{
    PreparedSurfaceGlyphFrame, SurfaceBackend, SurfaceConfigError, SurfaceConfigPlanner,
    SurfaceConfigurationController, SurfaceFrameBackend, SurfaceFrameError, SurfaceGlyphFrame,
    SurfaceLifecycleAction, WgpuRenderer,
};

use super::{NativeAppError, NativeGlyphFrameError, NativeTerminalRuntime};

/// Native window surface state owned by the app after a `wgpu` surface exists.
#[derive(Debug)]
pub struct NativeWindowSurface<B> {
    backend: B,
    capabilities: wgpu::SurfaceCapabilities,
    controller: SurfaceConfigurationController,
}

impl<B> NativeWindowSurface<B>
where
    B: SurfaceBackend,
{
    /// Create app-facing surface state for a concrete backend and capabilities.
    pub fn new(backend: B, capabilities: wgpu::SurfaceCapabilities) -> Self {
        Self {
            backend,
            capabilities,
            controller: SurfaceConfigurationController::new(SurfaceConfigPlanner::new()),
        }
    }

    /// Create and configure app-owned surface state from a GPU surface handoff.
    pub fn from_gpu_surface(
        gpu_surface: NativeGpuWindowSurface<B>,
        width: u32,
        height: u32,
    ) -> std::result::Result<Self, SurfaceConfigError> {
        let (backend, capabilities) = gpu_surface.into_parts();
        let mut surface = Self::new(backend, capabilities);
        surface.configure_initial(width, height)?;
        Ok(surface)
    }

    /// Configure the initial window surface size.
    pub fn configure_initial(
        &mut self,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError> {
        self.controller
            .configure(&mut self.backend, &self.capabilities, width, height)
    }

    /// Reconfigure the surface after a native resize when required.
    pub fn resize(
        &mut self,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError> {
        self.controller
            .resize(&mut self.backend, &self.capabilities, width, height)
    }

    /// Access the concrete surface backend.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Last configured non-zero surface size.
    pub fn configured_size(&self) -> Option<(u32, u32)> {
        self.controller.lifecycle().size()
    }

    /// Whether surface configuration is suspended for a zero-sized native window.
    pub fn is_suspended(&self) -> bool {
        self.controller.lifecycle().is_suspended()
    }

    /// Number of configure/reconfigure operations applied to the backend.
    pub fn configure_count(&self) -> u64 {
        self.controller.lifecycle().configure_count()
    }
}

impl<B> NativeWindowSurface<B>
where
    B: SurfaceFrameBackend,
{
    /// Clear the current native surface frame and present it.
    pub fn clear_and_present(
        &mut self,
        clear_color: [f64; 4],
    ) -> std::result::Result<(), SurfaceFrameError> {
        self.backend.clear_and_present(clear_color)
    }

    /// Render terminal glyph quads to the current native surface frame and present it.
    pub fn present_glyph_frame(
        &mut self,
        frame: SurfaceGlyphFrame<'_>,
    ) -> std::result::Result<(), SurfaceFrameError> {
        self.backend.present_glyph_frame(frame)
    }
}

/// Render dirty terminal state into a prepared glyph frame and present it through a native surface.
pub fn render_and_present_terminal_glyph_frame<S, B>(
    runtime: &mut NativeTerminalRuntime<S>,
    renderer: &mut WgpuRenderer,
    glyph_cache: &mut RasterizedGlyphCache,
    surface: &mut NativeWindowSurface<B>,
) -> Result<bool, NativeGlyphFrameError>
where
    B: SurfaceFrameBackend,
{
    if !runtime.render_terminal_frame(renderer)? {
        return Ok(false);
    }
    let clear_color = renderer.config().clear_color;
    let Some(plan) = renderer.last_plan() else {
        return Ok(false);
    };
    if plan.glyphs.is_empty() {
        surface.clear_and_present(clear_color)?;
        return Ok(false);
    }
    let glyphs = glyph_cache.rasterize_plan(plan)?;
    let prepared = PreparedSurfaceGlyphFrame::from_render_plan(plan, &glyphs.bitmaps, clear_color)?;
    surface.present_glyph_frame(prepared.as_surface_glyph_frame())?;
    Ok(true)
}

/// Build the default native glyph cache from a system monospace font.
pub fn load_default_native_glyph_cache() -> Result<RasterizedGlyphCache, NativeAppError> {
    for path in DEFAULT_MONOSPACE_FONT_CANDIDATES {
        if Path::new(path).exists() {
            let mut font_bytes = vec![
                std::fs::read(path).map_err(|error| NativeAppError::Runtime(error.to_string()))?,
            ];
            for fallback_path in DEFAULT_FALLBACK_FONT_CANDIDATES {
                if Path::new(fallback_path).exists() {
                    font_bytes.push(
                        std::fs::read(fallback_path)
                            .map_err(|error| NativeAppError::Runtime(error.to_string()))?,
                    );
                }
            }
            return RasterizedGlyphCache::from_font_bytes(font_bytes).map_err(NativeAppError::from);
        }
    }
    Err(NativeAppError::Runtime(
        "no default monospace system font found".to_owned(),
    ))
}

const DEFAULT_MONOSPACE_FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/SFNSMono.ttf",
    "/System/Library/Fonts/Menlo.ttc",
    "/System/Library/Fonts/Supplemental/Courier New.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/dejavu-sans-fonts/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
    "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
];

const DEFAULT_FALLBACK_FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/Apple Color Emoji.ttc",
    "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
];
