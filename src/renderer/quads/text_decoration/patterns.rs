use super::geometry::{DecorationGeometry, decoration_quad, decoration_segment_quad};
use super::{BackgroundQuad, BackgroundQuadError, TextDecorationQuadPlanner};

impl TextDecorationQuadPlanner {
    pub(super) fn append_dotted_underline(
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

    pub(super) fn append_dashed_underline(
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

    pub(super) fn append_curly_underline(
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
}
