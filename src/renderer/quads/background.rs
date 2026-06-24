use thiserror::Error;

use crate::renderer::color::rgba8_to_linear_normalized;
use crate::renderer::{PlannedBackground, RenderPlan};

/// Pixel layout used to build solid background quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundQuadConfig {
    /// Terminal cell width in pixels.
    pub cell_width_px: u32,
    /// Terminal cell height in pixels.
    pub cell_height_px: u32,
}

/// Errors produced while building solid background quads.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackgroundQuadError {
    /// Pixel dimensions must be non-zero.
    #[error("background quad dimensions must be non-zero")]
    ZeroDimension,
    /// The planned background batch cannot be represented in `u32` GPU indices.
    #[error("background quad count is too large for u32 GPU indices")]
    IndexCountTooLarge,
}

/// One vertex for a solid background quad.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BackgroundVertex {
    /// Pixel-space output position.
    pub position: [f32; 2],
    /// Solid background color in normalized RGBA.
    pub color_rgba: [f32; 4],
}

/// One solid background quad derived from styled terminal cells.
#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundQuad {
    /// Grid row represented by this quad.
    pub row: u16,
    /// Starting grid column represented by this quad.
    pub col: u16,
    /// Number of adjacent cells represented by this quad.
    pub cols: u16,
    /// Quad vertices in top-left, top-right, bottom-right, bottom-left order.
    pub vertices: [BackgroundVertex; 4],
}

/// Indexed solid background quad batch ready for GPU vertex/index buffer upload.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BackgroundQuadBatch {
    /// Solid background quads.
    pub quads: Vec<BackgroundQuad>,
    /// Triangle indices for all quads.
    pub indices: Vec<u32>,
}

/// Deterministic CPU-side planner for terminal background fill quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundQuadPlanner {
    config: BackgroundQuadConfig,
}

impl BackgroundQuadPlanner {
    /// Create a background quad planner.
    pub fn new(config: BackgroundQuadConfig) -> Self {
        Self { config }
    }

    /// Build solid background quads and triangle indices from a render plan.
    pub fn plan(
        &self,
        plan: &RenderPlan,
    ) -> std::result::Result<BackgroundQuadBatch, BackgroundQuadError> {
        self.validate_config()?;
        let mut quads = Vec::new();
        quads
            .try_reserve_exact(plan.backgrounds.len())
            .map_err(|_| BackgroundQuadError::IndexCountTooLarge)?;
        let mut indices = Vec::new();
        indices
            .try_reserve_exact(checked_background_quad_index_capacity(
                plan.backgrounds.len(),
            )?)
            .map_err(|_| BackgroundQuadError::IndexCountTooLarge)?;

        for background in &plan.backgrounds {
            let quad = self.plan_background(*background);
            let base = checked_background_quad_base_index(quads.len())?;
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            quads.push(quad);
        }

        Ok(BackgroundQuadBatch { quads, indices })
    }

    fn validate_config(&self) -> std::result::Result<(), BackgroundQuadError> {
        if self.config.cell_width_px == 0 || self.config.cell_height_px == 0 {
            return Err(BackgroundQuadError::ZeroDimension);
        }
        Ok(())
    }

    fn plan_background(&self, background: PlannedBackground) -> BackgroundQuad {
        let cell_width = self.config.cell_width_px as f32;
        let cell_height = self.config.cell_height_px as f32;
        let x0 = f32::from(background.col) * cell_width;
        let y0 = f32::from(background.row) * cell_height;
        let x1 = x0 + (cell_width * f32::from(background.cols));
        let y1 = y0 + cell_height;
        let color_rgba = rgba8_to_linear_normalized(background.color_rgba8);

        BackgroundQuad {
            row: background.row,
            col: background.col,
            cols: background.cols,
            vertices: [
                BackgroundVertex {
                    position: [x0, y0],
                    color_rgba,
                },
                BackgroundVertex {
                    position: [x1, y0],
                    color_rgba,
                },
                BackgroundVertex {
                    position: [x1, y1],
                    color_rgba,
                },
                BackgroundVertex {
                    position: [x0, y1],
                    color_rgba,
                },
            ],
        }
    }
}

pub(in crate::renderer::quads) fn checked_background_quad_base_index(
    quad_index: usize,
) -> std::result::Result<u32, BackgroundQuadError> {
    u32::try_from(quad_index)
        .ok()
        .and_then(|index| index.checked_mul(4))
        .ok_or(BackgroundQuadError::IndexCountTooLarge)
}

pub(in crate::renderer::quads) fn checked_background_quad_index_capacity(
    quad_count: usize,
) -> std::result::Result<usize, BackgroundQuadError> {
    quad_count
        .checked_mul(6)
        .ok_or(BackgroundQuadError::IndexCountTooLarge)
}
