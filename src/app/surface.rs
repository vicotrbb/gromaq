use std::path::{Path, PathBuf};

use crate::config::DEFAULT_FONT_FAMILY;
use crate::font::RasterizedGlyphCache;
use crate::native_gpu::NativeGpuWindowSurface;
use crate::renderer::{
    PreparedSurfaceGlyphFrame, SurfaceBackend, SurfaceConfigError, SurfaceConfigPlanner,
    SurfaceConfigurationController, SurfaceFrameBackend, SurfaceFrameError, SurfaceGlyphFrame,
    SurfaceLifecycleAction, WgpuRenderer,
};

use super::{NativeAppError, NativeGlyphFrameError, NativeTerminalRuntime};

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

/// Build a native glyph cache from a configured font path or the default system monospace font.
pub fn load_native_glyph_cache(font_family: &str) -> Result<RasterizedGlyphCache, NativeAppError> {
    let font_family = font_family.trim();
    if let Some(path) = configured_font_file_path(font_family)? {
        return load_glyph_cache_from_primary_font(path);
    }
    if is_default_font_family(font_family) {
        return load_default_native_glyph_cache();
    }
    if let Some(path) = resolve_named_font_file_path(font_family) {
        return load_glyph_cache_from_primary_font(&path);
    }
    Err(NativeAppError::Runtime(format!(
        "configured font family is not installed or supported by name: {font_family}; use an explicit font file path"
    )))
}

/// Build the default native glyph cache from a system monospace font.
pub fn load_default_native_glyph_cache() -> Result<RasterizedGlyphCache, NativeAppError> {
    if let Some(path) = first_existing_font_path(default_monospace_font_candidate_paths()) {
        return load_glyph_cache_from_primary_font(&path);
    }
    Err(NativeAppError::Runtime(
        "no default monospace system font found".to_owned(),
    ))
}

fn configured_font_file_path(font_family: &str) -> Result<Option<&Path>, NativeAppError> {
    let path = Path::new(font_family);
    if path.is_file() {
        return Ok(Some(path));
    }
    if path.is_absolute() || font_family.contains('/') || font_family.contains('\\') {
        return Err(NativeAppError::Runtime(format!(
            "configured font file does not exist: {font_family}"
        )));
    }
    Ok(None)
}

fn is_default_font_family(font_family: &str) -> bool {
    font_family.is_empty() || font_family.eq_ignore_ascii_case(DEFAULT_FONT_FAMILY)
}

fn resolve_named_font_file_path(font_family: &str) -> Option<PathBuf> {
    first_existing_font_path(named_font_candidate_paths(font_family)?)
}

fn first_existing_font_path(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|path| path.exists())
}

fn default_monospace_font_candidate_paths() -> Vec<PathBuf> {
    let mut candidates = font_search_candidates(DEFAULT_PREFERRED_MONO_FONT_FILES);
    candidates.extend(DEFAULT_MONOSPACE_FONT_CANDIDATES.iter().map(PathBuf::from));
    candidates
}

fn named_font_candidate_paths(font_family: &str) -> Option<Vec<PathBuf>> {
    let files = match normalized_font_family_name(font_family).as_str() {
        "jetbrainsmono" | "jetbrainsmononerdfont" => JETBRAINS_MONO_FONT_FILES,
        "cascadiamono" | "caskaydiacovenerdfont" => CASCADIA_MONO_FONT_FILES,
        "iosevkaterm" | "iosevka" => IOSEVKA_TERM_FONT_FILES,
        "sfmono" => SF_MONO_FONT_FILES,
        "menlo" => MENLO_FONT_FILES,
        _ => return None,
    };
    Some(font_search_candidates(files))
}

fn normalized_font_family_name(font_family: &str) -> String {
    font_family
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn font_search_candidates(file_names: &[&str]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for root in font_search_roots() {
        for file_name in file_names {
            candidates.push(root.join(file_name));
        }
    }
    candidates
}

fn font_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join("Library/Fonts"));
    }
    roots.extend([
        PathBuf::from("/Library/Fonts"),
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/opt/homebrew/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
        PathBuf::from("/usr/share/fonts/truetype"),
        PathBuf::from("/usr/share/fonts/opentype"),
    ]);
    roots
}

fn load_glyph_cache_from_primary_font(path: &Path) -> Result<RasterizedGlyphCache, NativeAppError> {
    let mut font_bytes =
        vec![std::fs::read(path).map_err(|error| NativeAppError::Runtime(error.to_string()))?];
    for fallback_path in DEFAULT_FALLBACK_FONT_CANDIDATES
        .iter()
        .map(Path::new)
        .filter(|path| path.exists())
    {
        font_bytes.push(
            std::fs::read(fallback_path)
                .map_err(|error| NativeAppError::Runtime(error.to_string()))?,
        );
    }
    RasterizedGlyphCache::from_font_bytes(font_bytes).map_err(NativeAppError::from)
}

const DEFAULT_PREFERRED_MONO_FONT_FILES: &[&str] = &[
    "JetBrainsMonoNerdFont-Regular.ttf",
    "JetBrainsMonoNLNerdFont-Regular.ttf",
    "JetBrainsMono-Regular.ttf",
    "CaskaydiaCoveNerdFont-Regular.ttf",
    "CascadiaMono.ttf",
    "IosevkaTerm-Regular.ttf",
    "SFNSMono.ttf",
    "Menlo.ttc",
];

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

const JETBRAINS_MONO_FONT_FILES: &[&str] = &[
    "JetBrainsMonoNerdFont-Regular.ttf",
    "JetBrainsMonoNLNerdFont-Regular.ttf",
    "JetBrainsMono-Regular.ttf",
];

const CASCADIA_MONO_FONT_FILES: &[&str] = &[
    "CaskaydiaCoveNerdFont-Regular.ttf",
    "CascadiaMono.ttf",
    "CascadiaCode.ttf",
];

const IOSEVKA_TERM_FONT_FILES: &[&str] = &[
    "IosevkaTerm-Regular.ttf",
    "IosevkaTermNerdFont-Regular.ttf",
    "Iosevka-Regular.ttc",
];

const SF_MONO_FONT_FILES: &[&str] = &["SFNSMono.ttf", "SFNSMonoItalic.ttf"];

const MENLO_FONT_FILES: &[&str] = &["Menlo.ttc"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_font_stack_prefers_polished_user_fonts_before_system_fallbacks() {
        let candidates = default_monospace_font_candidate_paths();
        let names = candidates
            .iter()
            .filter_map(|path| path.file_name())
            .map(|name| name.to_string_lossy())
            .collect::<Vec<_>>();

        let jetbrains_index = names
            .iter()
            .position(|name| name == "JetBrainsMonoNerdFont-Regular.ttf")
            .unwrap();
        let sf_mono_index = names
            .iter()
            .position(|name| name == "SFNSMono.ttf")
            .unwrap();

        assert!(jetbrains_index < sf_mono_index);
    }

    #[test]
    fn named_font_resolution_normalizes_common_family_names() {
        let candidates = named_font_candidate_paths("JetBrains Mono Nerd Font").unwrap();
        let names = candidates
            .iter()
            .filter_map(|path| path.file_name())
            .map(|name| name.to_string_lossy())
            .collect::<Vec<_>>();

        assert!(names.contains(&"JetBrainsMonoNerdFont-Regular.ttf".into()));
        assert!(named_font_candidate_paths("Unmapped Mono").is_none());
    }
}
