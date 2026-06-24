use crate::terminal::{CursorShape, CursorSnapshot};

use super::{BackgroundQuad, BackgroundQuadBatch, BackgroundQuadError, BackgroundVertex};
use crate::renderer::RenderPlan;
use crate::renderer::color::rgba8_to_normalized;

/// Pixel layout used to build solid cursor quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorQuadConfig {
    /// Terminal cell width in pixels.
    pub cell_width_px: u32,
    /// Terminal cell height in pixels.
    pub cell_height_px: u32,
    /// Cursor color in RGBA8.
    pub color_rgba8: [u8; 4],
}

/// Deterministic CPU-side planner for terminal cursor quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorQuadPlanner {
    config: CursorQuadConfig,
}

impl CursorQuadPlanner {
    /// Create a cursor quad planner.
    pub fn new(config: CursorQuadConfig) -> Self {
        Self { config }
    }

    /// Build a solid cursor quad batch from a render plan.
    pub fn plan(
        &self,
        plan: &RenderPlan,
    ) -> std::result::Result<BackgroundQuadBatch, BackgroundQuadError> {
        self.validate_config()?;
        if !plan.cursor.visible
            || plan.cursor.row >= plan.viewport_rows
            || plan.cursor.col >= plan.viewport_cols
        {
            return Ok(BackgroundQuadBatch::default());
        }
        let quad = self.plan_cursor(plan.cursor);
        Ok(BackgroundQuadBatch {
            quads: vec![quad],
            indices: vec![0, 1, 2, 0, 2, 3],
        })
    }

    fn validate_config(&self) -> std::result::Result<(), BackgroundQuadError> {
        if self.config.cell_width_px == 0 || self.config.cell_height_px == 0 {
            return Err(BackgroundQuadError::ZeroDimension);
        }
        Ok(())
    }

    fn plan_cursor(&self, cursor: CursorSnapshot) -> BackgroundQuad {
        let cell_width = self.config.cell_width_px as f32;
        let cell_height = self.config.cell_height_px as f32;
        let cell_x0 = f32::from(cursor.col) * cell_width;
        let cell_y0 = f32::from(cursor.row) * cell_height;
        let cell_x1 = cell_x0 + cell_width;
        let cell_y1 = cell_y0 + cell_height;
        let (x0, y0, x1, y1) = match cursor.shape {
            CursorShape::Block => (cell_x0, cell_y0, cell_x1, cell_y1),
            CursorShape::Underline => {
                let thickness = cursor_stroke_px(cell_height);
                (cell_x0, cell_y1 - thickness, cell_x1, cell_y1)
            }
            CursorShape::Bar => {
                let thickness = cursor_stroke_px(cell_width);
                (cell_x0, cell_y0, cell_x0 + thickness, cell_y1)
            }
        };
        let color_rgba = rgba8_to_normalized(self.config.color_rgba8);

        BackgroundQuad {
            row: cursor.row,
            col: cursor.col,
            cols: 1,
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

fn cursor_stroke_px(cell_dimension: f32) -> f32 {
    (cell_dimension / 8.0).ceil().clamp(1.0, cell_dimension)
}
