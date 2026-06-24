//! GPU renderer boundary.

use crate::config::GromaqConfig;
use crate::dirty::DirtyRegion;
use crate::error::Result;
use crate::grid::GridSnapshot;
use crate::terminal::CursorSnapshot;

mod atlas;
mod color;
mod plan;
mod prepared_frame;
mod quads;
mod scheduler;
mod surface;
mod surface_buffers;
mod surface_frame;

pub use atlas::{
    GlyphAtlas, GlyphAtlasConfig, GlyphAtlasImage, GlyphAtlasMetrics, GlyphBitmap, GlyphEntry,
    GlyphImageError, GlyphKey, GlyphKeyText,
};
pub use plan::{
    PlannedBackground, PlannedGlyph, PlannedTextDecoration, RenderPlan, RenderPlanner,
    TextDecorationKind,
};
pub use prepared_frame::{PreparedSurfaceGlyphFrame, SurfaceGlyphFrame};
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
#[cfg(test)]
use surface_buffers::{
    SurfaceGlyphAtlasLayout, SurfaceGlyphBufferLayout, surface_background_vertex_byte_capacity,
    surface_glyph_vertex_byte_capacity, validate_surface_background_buffers,
    validate_surface_glyph_buffers, validate_surface_glyph_frame,
};
pub use surface_frame::{SurfaceFrameBackend, SurfaceFrameError, WgpuSurfaceBackend};

const DEFAULT_RENDERER_FONT_SIZE_PX: u16 = 14;
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
    /// Clear color in RGBA linear space.
    pub clear_color: [f64; 4],
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            target_fps: 144,
            dirty_regions: true,
            font_size_px: DEFAULT_RENDERER_FONT_SIZE_PX,
            clear_color: [0.02, 0.02, 0.025, 1.0],
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
            ..Self::default()
        })
    }
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
            planner: RenderPlanner::new(config.font_size_px),
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
        self.planner = RenderPlanner::new(config.font_size_px);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn one_quad_batch() -> GlyphQuadBatch {
        let quad = GlyphQuad {
            text: "A".to_owned(),
            ch: 'A',
            atlas_entry: GlyphEntry {
                slot: 0,
                generation: 0,
            },
            vertices: [
                GlyphVertex {
                    position: [0.0, 0.0],
                    uv: [0.0, 0.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                GlyphVertex {
                    position: [1.0, 0.0],
                    uv: [1.0, 0.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                GlyphVertex {
                    position: [1.0, 1.0],
                    uv: [1.0, 1.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                GlyphVertex {
                    position: [0.0, 1.0],
                    uv: [0.0, 1.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
            ],
        };
        GlyphQuadBatch {
            quads: vec![quad],
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    }

    #[test]
    fn frame_scheduler_dropped_frame_metrics_saturate() {
        let mut scheduler = FrameScheduler::new(1).unwrap();
        scheduler.metrics_mut().dropped_frames = u64::MAX - 1;
        let start = Instant::now();
        scheduler.record_presented(start);

        scheduler.record_presented(start + Duration::from_secs(4));

        assert_eq!(scheduler.metrics().dropped_frames, u64::MAX);
    }

    #[test]
    fn frame_scheduler_presented_frame_metrics_saturate() {
        let mut scheduler = FrameScheduler::new(144).unwrap();
        scheduler.metrics_mut().frames_presented = u64::MAX;

        scheduler.record_presented(Instant::now());

        assert_eq!(scheduler.metrics().frames_presented, u64::MAX);
    }

    #[test]
    fn surface_glyph_frame_validation_computes_checked_atlas_layout() {
        let atlas = GlyphAtlasImage {
            width: 2,
            height: 2,
            rgba: vec![255; 16],
            occupied_slots: 1,
        };
        let batch = one_quad_batch();

        let layout = validate_surface_glyph_frame(SurfaceGlyphFrame {
            atlas: &atlas,
            background_batch: &BackgroundQuadBatch::default(),
            batch: &batch,
            decoration_batch: &BackgroundQuadBatch::default(),
            cursor_batch: &BackgroundQuadBatch::default(),
            width: 16,
            height: 16,
            clear_color: [0.0, 0.0, 0.0, 1.0],
        })
        .unwrap();

        assert_eq!(
            layout,
            SurfaceGlyphAtlasLayout {
                row_bytes: 8,
                expected_len: 16,
            }
        );
    }

    #[test]
    fn surface_glyph_frame_validation_rejects_overflowing_atlas_row_size() {
        let atlas = GlyphAtlasImage {
            width: u32::MAX,
            height: 1,
            rgba: Vec::new(),
            occupied_slots: 0,
        };
        let batch = one_quad_batch();

        let error = validate_surface_glyph_frame(SurfaceGlyphFrame {
            atlas: &atlas,
            background_batch: &BackgroundQuadBatch::default(),
            batch: &batch,
            decoration_batch: &BackgroundQuadBatch::default(),
            cursor_batch: &BackgroundQuadBatch::default(),
            width: 16,
            height: 16,
            clear_color: [0.0, 0.0, 0.0, 1.0],
        })
        .unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame("surface glyph atlas row size is too large".to_owned())
        );
    }

    #[test]
    fn surface_glyph_frame_validation_accepts_background_only_batches() {
        let atlas = GlyphAtlasImage {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
            occupied_slots: 0,
        };
        let background_batch = BackgroundQuadBatch {
            quads: vec![BackgroundQuad {
                row: 0,
                col: 0,
                cols: 1,
                vertices: [
                    BackgroundVertex {
                        position: [0.0, 0.0],
                        color_rgba: [1.0, 0.0, 0.0, 1.0],
                    },
                    BackgroundVertex {
                        position: [1.0, 0.0],
                        color_rgba: [1.0, 0.0, 0.0, 1.0],
                    },
                    BackgroundVertex {
                        position: [1.0, 1.0],
                        color_rgba: [1.0, 0.0, 0.0, 1.0],
                    },
                    BackgroundVertex {
                        position: [0.0, 1.0],
                        color_rgba: [1.0, 0.0, 0.0, 1.0],
                    },
                ],
            }],
            indices: vec![0, 1, 2, 0, 2, 3],
        };

        let layout = validate_surface_glyph_frame(SurfaceGlyphFrame {
            atlas: &atlas,
            background_batch: &background_batch,
            batch: &GlyphQuadBatch::default(),
            decoration_batch: &BackgroundQuadBatch::default(),
            cursor_batch: &BackgroundQuadBatch::default(),
            width: 1,
            height: 1,
            clear_color: [0.0, 0.0, 0.0, 1.0],
        })
        .unwrap();

        assert_eq!(
            layout,
            SurfaceGlyphAtlasLayout {
                row_bytes: 4,
                expected_len: 4,
            }
        );
    }

    #[test]
    fn surface_glyph_frame_validation_accepts_cursor_only_batches() {
        let atlas = GlyphAtlasImage {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
            occupied_slots: 0,
        };
        let cursor_batch = BackgroundQuadBatch {
            quads: vec![BackgroundQuad {
                row: 0,
                col: 0,
                cols: 1,
                vertices: [
                    BackgroundVertex {
                        position: [0.0, 0.0],
                        color_rgba: [1.0, 1.0, 1.0, 1.0],
                    },
                    BackgroundVertex {
                        position: [1.0, 0.0],
                        color_rgba: [1.0, 1.0, 1.0, 1.0],
                    },
                    BackgroundVertex {
                        position: [1.0, 1.0],
                        color_rgba: [1.0, 1.0, 1.0, 1.0],
                    },
                    BackgroundVertex {
                        position: [0.0, 1.0],
                        color_rgba: [1.0, 1.0, 1.0, 1.0],
                    },
                ],
            }],
            indices: vec![0, 1, 2, 0, 2, 3],
        };

        let layout = validate_surface_glyph_frame(SurfaceGlyphFrame {
            atlas: &atlas,
            background_batch: &BackgroundQuadBatch::default(),
            batch: &GlyphQuadBatch::default(),
            decoration_batch: &BackgroundQuadBatch::default(),
            cursor_batch: &cursor_batch,
            width: 1,
            height: 1,
            clear_color: [0.0, 0.0, 0.0, 1.0],
        })
        .unwrap();

        assert_eq!(
            layout,
            SurfaceGlyphAtlasLayout {
                row_bytes: 4,
                expected_len: 4,
            }
        );
    }

    #[test]
    fn surface_glyph_buffer_validation_reports_checked_sizes() {
        let vertex_bytes = [1_u8, 2, 3, 4];
        let index_bytes = [5_u8, 6, 7, 8];

        let layout = validate_surface_glyph_buffers(&vertex_bytes, &index_bytes, 1).unwrap();

        assert_eq!(
            layout,
            SurfaceGlyphBufferLayout {
                vertex_buffer_size: 4,
                index_buffer_size: 4,
                index_count: 1,
            }
        );
    }

    #[test]
    fn surface_background_buffer_validation_reports_checked_sizes() {
        let vertex_bytes = [1_u8, 2, 3, 4];
        let index_bytes = [5_u8, 6, 7, 8];

        let layout = validate_surface_background_buffers(&vertex_bytes, &index_bytes, 1).unwrap();

        assert_eq!(
            layout,
            SurfaceGlyphBufferLayout {
                vertex_buffer_size: 4,
                index_buffer_size: 4,
                index_count: 1,
            }
        );
    }

    #[test]
    fn surface_glyph_vertex_byte_capacity_uses_checked_multiplication() {
        assert_eq!(surface_glyph_vertex_byte_capacity(2).unwrap(), 256);

        let error = surface_glyph_vertex_byte_capacity((usize::MAX / 128) + 1).unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame("surface glyph vertex bytes are too large".to_owned())
        );
    }

    #[test]
    fn surface_background_vertex_byte_capacity_uses_checked_multiplication() {
        assert_eq!(surface_background_vertex_byte_capacity(2).unwrap(), 192);

        let error = surface_background_vertex_byte_capacity((usize::MAX / 96) + 1).unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame(
                "surface background vertex bytes are too large".to_owned()
            )
        );
    }

    #[test]
    fn surface_glyph_buffer_validation_rejects_empty_buffers() {
        let vertex_bytes = [];
        let index_bytes = [1_u8, 2, 3, 4];

        let error = validate_surface_glyph_buffers(&vertex_bytes, &index_bytes, 1).unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame(
                "surface glyph draw buffers must be non-empty".to_owned()
            )
        );
    }

    #[test]
    fn surface_background_buffer_validation_rejects_empty_buffers() {
        let vertex_bytes = [];
        let index_bytes = [1_u8, 2, 3, 4];

        let error =
            validate_surface_background_buffers(&vertex_bytes, &index_bytes, 1).unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame(
                "surface background draw buffers must be non-empty".to_owned()
            )
        );
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn surface_glyph_buffer_validation_rejects_oversized_index_count() {
        let vertex_bytes = [1_u8, 2, 3, 4];
        let index_bytes = [5_u8, 6, 7, 8];

        let error =
            validate_surface_glyph_buffers(&vertex_bytes, &index_bytes, usize::MAX).unwrap_err();

        assert_eq!(
            error,
            SurfaceFrameError::InvalidFrame("surface glyph index count is too large".to_owned())
        );
    }
}
