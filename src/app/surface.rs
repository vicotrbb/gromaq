use crate::font::RasterizedGlyphCache;
use crate::native_gpu::NativeGpuWindowSurface;
use crate::renderer::{
    PreparedSurfaceGlyphFrame, SurfaceBackend, SurfaceConfigError, SurfaceConfigPlanner,
    SurfaceConfigurationController, SurfaceFrameBackend, SurfaceFrameError, SurfaceGlyphFrame,
    SurfaceLifecycleAction, WgpuRenderer,
};

use super::{NativeGlyphFrameError, NativeTerminalRuntime};

/// Structured result from preparing and presenting a native terminal glyph frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeGlyphFramePresentation {
    /// Whether dirty terminal state was rendered through the renderer boundary.
    pub rendered: bool,
    /// Whether a glyph frame was presented through the native surface backend.
    pub glyph_frame_presented: bool,
    /// Whether the surface was cleared without a glyph frame.
    pub clear_presented: bool,
    /// Presented frame width in pixels.
    pub width: u32,
    /// Presented frame height in pixels.
    pub height: u32,
    /// Textured glyph quads prepared for presentation.
    pub glyph_quads: usize,
    /// Solid background quads prepared for presentation.
    pub background_quads: usize,
    /// Solid text-decoration quads prepared for presentation.
    pub decoration_quads: usize,
    /// Solid cursor quads prepared for presentation.
    pub cursor_quads: usize,
    /// Packed glyph atlas byte length.
    pub atlas_bytes: usize,
    /// Occupied glyph atlas slots.
    pub atlas_occupied_slots: usize,
}

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

    /// Last configured native presentation mode.
    pub fn present_mode(&self) -> Option<wgpu::PresentMode> {
        self.controller
            .lifecycle()
            .current_config()
            .map(|config| config.present_mode)
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
    render_and_present_terminal_glyph_frame_report(runtime, renderer, glyph_cache, surface)
        .map(|report| report.glyph_frame_presented)
}

/// Render dirty terminal state into a prepared glyph frame, present it, and return presentation metrics.
pub fn render_and_present_terminal_glyph_frame_report<S, B>(
    runtime: &mut NativeTerminalRuntime<S>,
    renderer: &mut WgpuRenderer,
    glyph_cache: &mut RasterizedGlyphCache,
    surface: &mut NativeWindowSurface<B>,
) -> Result<NativeGlyphFramePresentation, NativeGlyphFrameError>
where
    B: SurfaceFrameBackend,
{
    // Swapchain frames are not retained. Until native partial-present support exists,
    // every surface presentation must redraw the full visible terminal contents.
    runtime.invalidate_terminal_frame();
    if !runtime.render_terminal_frame(renderer)? {
        return Ok(NativeGlyphFramePresentation::default());
    }
    let clear_color = renderer.config().clear_color;
    let Some(plan) = renderer.last_plan() else {
        return Ok(NativeGlyphFramePresentation {
            rendered: true,
            ..NativeGlyphFramePresentation::default()
        });
    };
    let glyphs = glyph_cache.rasterize_plan(plan)?;
    let prepared = PreparedSurfaceGlyphFrame::from_render_plan(
        plan,
        &glyphs.bitmaps,
        renderer.config().cell_width_px,
        renderer.config().line_height_px,
        clear_color,
        renderer.config().cursor_color_rgba8,
        renderer.config().surface_padding_px,
    )?;
    let frame = prepared.as_surface_glyph_frame();
    let report = NativeGlyphFramePresentation {
        rendered: true,
        glyph_frame_presented: true,
        clear_presented: false,
        width: frame.width,
        height: frame.height,
        glyph_quads: frame.batch.quads.len(),
        background_quads: frame.background_batch.quads.len(),
        decoration_quads: frame.decoration_batch.quads.len(),
        cursor_quads: frame.cursor_batch.quads.len(),
        atlas_bytes: frame.atlas.rgba.len(),
        atlas_occupied_slots: frame.atlas.occupied_slots,
    };
    surface.present_glyph_frame(frame)?;
    Ok(report)
}
