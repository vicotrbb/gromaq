//! GPU renderer boundary.

use crate::dirty::DirtyRegion;
use crate::error::Result;
use crate::grid::GridSnapshot;
use crate::terminal::CursorSnapshot;

mod atlas;
mod color;
mod plan;
mod prepared_frame;
mod prepared_frame_atlas;
mod prepared_frame_geometry;
mod prepared_frame_preview;
mod quads;
mod scheduler;
mod settings;
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
pub use settings::RendererConfig;
pub use surface::{
    SurfaceBackend, SurfaceConfigError, SurfaceConfigPlanner, SurfaceConfigurationController,
    SurfaceLifecycle, SurfaceLifecycleAction,
};
pub use surface_frame::{SurfaceFrameBackend, SurfaceFrameError, WgpuSurfaceBackend};

const DEFAULT_GLYPH_ATLAS_CAPACITY: usize = 4096;

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
