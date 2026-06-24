use super::{
    BackgroundQuad, BackgroundQuadBatch, BackgroundQuadError, BackgroundVertex,
    checked_background_quad_base_index, checked_background_quad_index_capacity,
};
use crate::renderer::color::rgba8_to_normalized;
use crate::renderer::{PlannedTextDecoration, RenderPlan, TextDecorationKind};

/// Pixel layout used to build solid text-decoration quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextDecorationQuadConfig {
    /// Terminal cell width in pixels.
    pub cell_width_px: u32,
    /// Terminal cell height in pixels.
    pub cell_height_px: u32,
}

/// Deterministic CPU-side planner for straight terminal text-decoration quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextDecorationQuadPlanner {
    config: TextDecorationQuadConfig,
}

impl TextDecorationQuadPlanner {
    /// Create a text-decoration quad planner.
    pub fn new(config: TextDecorationQuadConfig) -> Self {
        Self { config }
    }

    /// Build solid decoration quads and triangle indices from a render plan.
    pub fn plan(
        &self,
        plan: &RenderPlan,
    ) -> std::result::Result<BackgroundQuadBatch, BackgroundQuadError> {
        self.validate_config()?;
        let mut quads = Vec::new();
        quads
            .try_reserve_exact(plan.decorations.len())
            .map_err(|_| BackgroundQuadError::IndexCountTooLarge)?;
        let mut indices = Vec::new();
        indices
            .try_reserve_exact(checked_background_quad_index_capacity(
                plan.decorations.len(),
            )?)
            .map_err(|_| BackgroundQuadError::IndexCountTooLarge)?;

        for decoration in &plan.decorations {
            self.append_decoration(*decoration, &mut quads, &mut indices)?;
        }

        Ok(BackgroundQuadBatch { quads, indices })
    }

    fn validate_config(&self) -> std::result::Result<(), BackgroundQuadError> {
        if self.config.cell_width_px == 0 || self.config.cell_height_px == 0 {
            return Err(BackgroundQuadError::ZeroDimension);
        }
        Ok(())
    }

    fn append_decoration(
        &self,
        decoration: PlannedTextDecoration,
        quads: &mut Vec<BackgroundQuad>,
        indices: &mut Vec<u32>,
    ) -> std::result::Result<(), BackgroundQuadError> {
        let cell_width = self.config.cell_width_px as f32;
        let cell_height = self.config.cell_height_px as f32;
        let x0 = f32::from(decoration.col) * cell_width;
        let x1 = x0 + (cell_width * f32::from(decoration.cols));
        let row_y0 = f32::from(decoration.row) * cell_height;
        let row_y1 = row_y0 + cell_height;
        let thickness = text_decoration_stroke_px(cell_height);
        let gap = thickness;
        let color_rgba = rgba8_to_normalized(decoration.color_rgba8);
        let geometry = DecorationGeometry {
            decoration,
            x0,
            x1,
            row_y1,
            thickness,
            color_rgba,
            cell_width,
        };
        match decoration.kind {
            TextDecorationKind::Underline | TextDecorationKind::DoubleUnderlineBottom => self
                .push_decoration_quad(
                    quads,
                    indices,
                    decoration_quad(decoration, x0, x1, row_y1 - thickness, row_y1, color_rgba),
                ),
            TextDecorationKind::DoubleUnderlineTop => self.push_decoration_quad(
                quads,
                indices,
                decoration_quad(
                    decoration,
                    x0,
                    x1,
                    row_y1 - (thickness * 2.0) - gap,
                    row_y1 - thickness - gap,
                    color_rgba,
                ),
            ),
            TextDecorationKind::CurlyUnderline => {
                self.append_curly_underline(quads, indices, geometry)
            }
            TextDecorationKind::DottedUnderline => {
                self.append_dotted_underline(quads, indices, geometry)
            }
            TextDecorationKind::DashedUnderline => {
                self.append_dashed_underline(quads, indices, geometry)
            }
            TextDecorationKind::Overline => self.push_decoration_quad(
                quads,
                indices,
                decoration_quad(decoration, x0, x1, row_y0, row_y0 + thickness, color_rgba),
            ),
            TextDecorationKind::Strikethrough => {
                let center = row_y0 + (cell_height / 2.0);
                let y0 = center - (thickness / 2.0);
                self.push_decoration_quad(
                    quads,
                    indices,
                    decoration_quad(decoration, x0, x1, y0, y0 + thickness, color_rgba),
                )
            }
        }
    }

    fn append_dotted_underline(
        &self,
        quads: &mut Vec<BackgroundQuad>,
        indices: &mut Vec<u32>,
        geometry: DecorationGeometry,
    ) -> std::result::Result<(), BackgroundQuadError> {
        let dot_size = geometry.thickness;
        let advance = dot_size * 2.0;
        let mut x = geometry.x0;
        while x < geometry.x1 {
            let dot_x1 = (x + dot_size).min(geometry.x1);
            self.push_decoration_quad(
                quads,
                indices,
                decoration_quad(
                    geometry.decoration,
                    x,
                    dot_x1,
                    geometry.row_y1 - geometry.thickness,
                    geometry.row_y1,
                    geometry.color_rgba,
                ),
            )?;
            x += advance;
        }
        Ok(())
    }

    fn append_dashed_underline(
        &self,
        quads: &mut Vec<BackgroundQuad>,
        indices: &mut Vec<u32>,
        geometry: DecorationGeometry,
    ) -> std::result::Result<(), BackgroundQuadError> {
        let dash_width = (geometry.cell_width * 0.75).max(geometry.thickness * 2.0);
        let advance = dash_width + (geometry.thickness * 2.0);
        let mut x = geometry.x0;
        while x < geometry.x1 {
            let dash_x1 = (x + dash_width).min(geometry.x1);
            self.push_decoration_quad(
                quads,
                indices,
                decoration_quad(
                    geometry.decoration,
                    x,
                    dash_x1,
                    geometry.row_y1 - geometry.thickness,
                    geometry.row_y1,
                    geometry.color_rgba,
                ),
            )?;
            x += advance;
        }
        Ok(())
    }

    fn append_curly_underline(
        &self,
        quads: &mut Vec<BackgroundQuad>,
        indices: &mut Vec<u32>,
        geometry: DecorationGeometry,
    ) -> std::result::Result<(), BackgroundQuadError> {
        let segment_width = (geometry.cell_width / 2.0).max(geometry.thickness * 2.0);
        let high_y = geometry.row_y1 - (geometry.thickness * 3.0);
        let low_y = geometry.row_y1 - geometry.thickness;
        let mut x = geometry.x0;
        let mut y0 = low_y;
        let mut y1 = high_y;
        while x < geometry.x1 {
            let next_x = (x + segment_width).min(geometry.x1);
            self.push_decoration_quad(
                quads,
                indices,
                decoration_segment_quad(
                    geometry.decoration,
                    [x, y0],
                    [next_x, y1],
                    geometry.thickness,
                    geometry.color_rgba,
                ),
            )?;
            x = next_x;
            std::mem::swap(&mut y0, &mut y1);
        }
        Ok(())
    }

    fn push_decoration_quad(
        &self,
        quads: &mut Vec<BackgroundQuad>,
        indices: &mut Vec<u32>,
        quad: BackgroundQuad,
    ) -> std::result::Result<(), BackgroundQuadError> {
        let base = checked_background_quad_base_index(quads.len())?;
        quads
            .try_reserve_exact(1)
            .map_err(|_| BackgroundQuadError::IndexCountTooLarge)?;
        indices
            .try_reserve_exact(6)
            .map_err(|_| BackgroundQuadError::IndexCountTooLarge)?;
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        quads.push(quad);
        Ok(())
    }
}

fn text_decoration_stroke_px(cell_height: f32) -> f32 {
    (cell_height / 10.0).ceil().clamp(1.0, cell_height)
}

#[derive(Debug, Clone, Copy)]
struct DecorationGeometry {
    decoration: PlannedTextDecoration,
    x0: f32,
    x1: f32,
    row_y1: f32,
    thickness: f32,
    color_rgba: [f32; 4],
    cell_width: f32,
}

fn decoration_quad(
    decoration: PlannedTextDecoration,
    x0: f32,
    x1: f32,
    y0: f32,
    y1: f32,
    color_rgba: [f32; 4],
) -> BackgroundQuad {
    BackgroundQuad {
        row: decoration.row,
        col: decoration.col,
        cols: decoration.cols,
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

fn decoration_segment_quad(
    decoration: PlannedTextDecoration,
    start: [f32; 2],
    end: [f32; 2],
    thickness: f32,
    color_rgba: [f32; 4],
) -> BackgroundQuad {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let length = (dx.mul_add(dx, dy * dy)).sqrt();
    let half_thickness = thickness / 2.0;
    let (normal_x, normal_y) = if length > 0.0 {
        (
            (-dy / length) * half_thickness,
            (dx / length) * half_thickness,
        )
    } else {
        (0.0, half_thickness)
    };
    BackgroundQuad {
        row: decoration.row,
        col: decoration.col,
        cols: decoration.cols,
        vertices: [
            BackgroundVertex {
                position: [start[0] + normal_x, start[1] + normal_y],
                color_rgba,
            },
            BackgroundVertex {
                position: [end[0] + normal_x, end[1] + normal_y],
                color_rgba,
            },
            BackgroundVertex {
                position: [end[0] - normal_x, end[1] - normal_y],
                color_rgba,
            },
            BackgroundVertex {
                position: [start[0] - normal_x, start[1] - normal_y],
                color_rgba,
            },
        ],
    }
}
