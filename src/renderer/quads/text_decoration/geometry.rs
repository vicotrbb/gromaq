use super::{BackgroundQuad, BackgroundVertex};
use crate::renderer::PlannedTextDecoration;

pub(super) fn text_decoration_stroke_px(cell_height: f32) -> f32 {
    (cell_height / 10.0).ceil().clamp(1.0, cell_height)
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DecorationGeometry {
    pub(super) decoration: PlannedTextDecoration,
    pub(super) x0: f32,
    pub(super) x1: f32,
    pub(super) row_y1: f32,
    pub(super) thickness: f32,
    pub(super) color_rgba: [f32; 4],
    pub(super) cell_width: f32,
}

pub(super) fn decoration_quad(
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

pub(super) fn decoration_segment_quad(
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
