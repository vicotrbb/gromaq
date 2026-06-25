//! GPU renderer boundary.

use crate::config::{
    DEFAULT_ANSI_COLORS_RGB8, DEFAULT_BACKGROUND_RGB8, DEFAULT_CELL_SPACING_PX,
    DEFAULT_CURSOR_RGB8, DEFAULT_DIM_OPACITY, DEFAULT_FOREGROUND_RGB8, DEFAULT_SELECTION_RGB8,
    DEFAULT_SURFACE_PADDING_PX, GromaqConfig,
};
use crate::dirty::DirtyRegion;
use crate::error::Result;
use crate::grid::GridSnapshot;
use crate::terminal::CursorSnapshot;

use color::rgb8_to_linear_clear_color;

mod atlas;
mod color;
mod plan;
mod prepared_frame;
mod prepared_frame_atlas;
mod prepared_frame_geometry;
mod prepared_frame_preview;
mod quads;
mod scheduler;
mod surface;
mod surface_buffers;
mod surface_frame;
#[cfg(test)]
mod tests;

pub use atlas::{
    GlyphAtlas, GlyphAtlasConfig, GlyphAtlasImage, GlyphAtlasMetrics, GlyphBitmap, GlyphEntry,
    GlyphImageError, GlyphKey, GlyphKeyText,
};
pub use plan::{
    PlannedBackground, PlannedGlyph, PlannedTextDecoration, RenderPlan, RenderPlanner,
    TextDecorationKind,
};
pub use prepared_frame::{
    PreparedSurfaceGlyphFrame, PreparedSurfaceGlyphFrameConfig, SurfaceGlyphFrame,
};
pub use prepared_frame_preview::PreparedFramePreview;
pub use quads::{
    BackgroundQuad, BackgroundQuadBatch, BackgroundQuadConfig, BackgroundQuadError,
    BackgroundQuadPlanner, BackgroundVertex, CursorQuadConfig, CursorQuadPlanner, GlyphQuad,
    GlyphQuadBatch, GlyphQuadConfig, GlyphQuadError, GlyphQuadPlanner, GlyphVertex,
    TextDecorationQuadConfig, TextDecorationQuadPlanner,
};
pub use scheduler::{FrameDecision, FrameScheduler, FrameSchedulerMetrics, RenderReason};
pub use surface::{
    SurfaceBackend, SurfaceConfigError, SurfaceConfigPlanner, SurfaceConfigurationController,
    SurfaceLifecycle, SurfaceLifecycleAction,
};
pub use surface_frame::{SurfaceFrameBackend, SurfaceFrameError, WgpuSurfaceBackend};

const DEFAULT_RENDERER_FONT_SIZE_PX: u16 = 37;
const DEFAULT_RENDERER_CELL_WIDTH_PX: u16 = 21;
const DEFAULT_GLYPH_ATLAS_CAPACITY: usize = 4096;

/// Renderer configuration for the GPU backend.
#[derive(Debug, Clone, PartialEq)]
pub struct RendererConfig {
    /// Target frames per second.
    pub target_fps: u32,
    /// Whether dirty-region rendering is required.
    pub dirty_regions: bool,
    /// Font size in pixels used for glyph planning and cache keys.
    pub font_size_px: u16,
    /// Terminal column width in pixels used for glyph, cursor, and input geometry.
    pub cell_width_px: u16,
    /// Terminal row height in pixels used for quad planning.
    pub line_height_px: u16,
    /// Clear color in RGBA linear space.
    pub clear_color: [f64; 4],
    /// Default foreground color for terminal cells with default SGR foreground.
    pub default_foreground_rgb8: [u8; 3],
    /// ANSI and bright ANSI color palette for terminal colors 0-15.
    pub ansi_colors_rgb8: [[u8; 3]; 16],
    /// Cursor color in RGBA8.
    pub cursor_color_rgba8: [u8; 4],
    /// Selection background color in RGBA8.
    pub selection_background_rgba8: [u8; 4],
    /// Empty space around rendered terminal cells in physical pixels.
    pub surface_padding_px: u16,
    /// Visual gap between adjacent rendered terminal cells in physical pixels.
    pub cell_spacing_px: u16,
    /// Opacity multiplier for SGR dim text.
    pub dim_opacity: f32,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            target_fps: 144,
            dirty_regions: true,
            font_size_px: DEFAULT_RENDERER_FONT_SIZE_PX,
            cell_width_px: DEFAULT_RENDERER_CELL_WIDTH_PX,
            line_height_px: 51,
            clear_color: rgb8_to_linear_clear_color(DEFAULT_BACKGROUND_RGB8),
            default_foreground_rgb8: DEFAULT_FOREGROUND_RGB8,
            ansi_colors_rgb8: DEFAULT_ANSI_COLORS_RGB8,
            cursor_color_rgba8: rgb8_to_rgba8(DEFAULT_CURSOR_RGB8),
            selection_background_rgba8: rgb8_to_rgba8(DEFAULT_SELECTION_RGB8),
            surface_padding_px: DEFAULT_SURFACE_PADDING_PX,
            cell_spacing_px: DEFAULT_CELL_SPACING_PX,
            dim_opacity: DEFAULT_DIM_OPACITY,
        }
    }
}

impl RendererConfig {
    /// Build renderer configuration from validated user configuration.
    pub fn from_gromaq_config(config: &GromaqConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            target_fps: config.performance.target_fps,
            dirty_regions: config.performance.dirty_region_rendering,
            font_size_px: config.font.renderer_font_size_px(),
            cell_width_px: config.font.renderer_cell_width_px(),
            line_height_px: config.font.renderer_line_height_px(),
            clear_color: rgb8_to_linear_clear_color(config.theme.background_rgb8()?),
            default_foreground_rgb8: config.theme.foreground_rgb8()?,
            ansi_colors_rgb8: config.theme.ansi_rgb8()?,
            cursor_color_rgba8: rgb8_to_rgba8(config.theme.cursor_rgb8()?),
            selection_background_rgba8: rgb8_to_rgba8(config.theme.selection_rgb8()?),
            surface_padding_px: config.theme.surface_padding_px,
            cell_spacing_px: config.theme.cell_spacing_px,
            dim_opacity: config.theme.dim_opacity,
        })
    }
}

fn rgb8_to_rgba8([red, green, blue]: [u8; 3]) -> [u8; 4] {
    [red, green, blue, 255]
}

/// Narrow GPU rendering interface.
pub trait GpuRenderer {
    /// Queue a terminal snapshot for rendering.
    fn render_frame(
        &mut self,
        grid: &GridSnapshot,
        cursor: CursorSnapshot,
        dirty_regions: &[DirtyRegion],
    ) -> Result<()>;
}

/// `wgpu` backend marker and configuration holder.
#[derive(Debug)]
pub struct WgpuRenderer {
    config: RendererConfig,
    planner: RenderPlanner,
    glyph_atlas: GlyphAtlas,
    last_plan: Option<RenderPlan>,
}

impl WgpuRenderer {
    /// Create a renderer boundary. Device creation is part of the native UI bootstrap.
    pub fn new(config: RendererConfig) -> Result<Self> {
        let atlas_config = GlyphAtlasConfig::new(DEFAULT_GLYPH_ATLAS_CAPACITY)?;
        Ok(Self {
            planner: RenderPlanner::with_visual_theme(
                config.font_size_px,
                config.default_foreground_rgb8,
                config.ansi_colors_rgb8,
                config.selection_background_rgba8,
                config.dim_opacity,
            ),
            config,
            glyph_atlas: GlyphAtlas::new(atlas_config),
            last_plan: None,
        })
    }

    /// Access renderer configuration.
    pub fn config(&self) -> &RendererConfig {
        &self.config
    }

    /// Replace renderer configuration for future frame planning.
    pub fn reconfigure(&mut self, config: RendererConfig) {
        self.planner = RenderPlanner::with_visual_theme(
            config.font_size_px,
            config.default_foreground_rgb8,
            config.ansi_colors_rgb8,
            config.selection_background_rgba8,
            config.dim_opacity,
        );
        self.config = config;
        self.last_plan = None;
    }

    /// Return the most recent planned frame.
    pub fn last_plan(&self) -> Option<&RenderPlan> {
        self.last_plan.as_ref()
    }

    /// Return internal glyph atlas metrics.
    pub fn glyph_atlas_metrics(&self) -> GlyphAtlasMetrics {
        self.glyph_atlas.metrics()
    }
}

impl GpuRenderer for WgpuRenderer {
    fn render_frame(
        &mut self,
        grid: &GridSnapshot,
        cursor: CursorSnapshot,
        dirty_regions: &[DirtyRegion],
    ) -> Result<()> {
        let full_viewport;
        let regions = if self.config.dirty_regions {
            dirty_regions
        } else {
            full_viewport = [DirtyRegion {
                row: 0,
                col: 0,
                rows: grid.rows,
                cols: grid.cols,
            }];
            &full_viewport
        };
        let plan = self
            .planner
            .plan_frame(grid, cursor, regions, &mut self.glyph_atlas)?;
        self.last_plan = Some(plan);
        Ok(())
    }
}
