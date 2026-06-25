//! Geometry helpers for owned surface glyph-frame preparation.

use super::{BackgroundQuadBatch, BackgroundVertex, GlyphQuadBatch, GlyphVertex};
use crate::renderer::SurfaceFrameError;

pub(super) fn checked_surface_frame_pixel_dimension(
    label: &'static str,
    cells: u16,
    cell_size_px: u32,
    surface_padding_px: u16,
    cell_spacing_px: u16,
) -> std::result::Result<u32, SurfaceFrameError> {
    u32::from(cells)
        .checked_mul(cell_size_px)
        .and_then(|cell_pixels| {
            let gaps = u32::from(cells.saturating_sub(1));
            gaps.checked_mul(u32::from(cell_spacing_px))
                .and_then(|spacing_pixels| cell_pixels.checked_add(spacing_pixels))
        })
        .and_then(|cell_pixels| {
            u32::from(surface_padding_px)
                .checked_mul(2)
                .and_then(|padding_pixels| cell_pixels.checked_add(padding_pixels))
        })
        .ok_or_else(|| {
            SurfaceFrameError::InvalidFrame(format!("{label} is too large to represent"))
        })
}

pub(super) fn apply_glyph_cell_spacing(
    batch: &mut GlyphQuadBatch,
    cell_width_px: u32,
    cell_spacing_px: u16,
) {
    if cell_spacing_px == 0 || cell_width_px == 0 {
        return;
    }
    let spacing = f32::from(cell_spacing_px);
    let cell_width = cell_width_px as f32;
    for quad in &mut batch.quads {
        let x0 = quad.vertices[0].position[0];
        let x1 = quad.vertices[1].position[0];
        let col = (x0 / cell_width).round().max(0.0);
        let span_cells = ((x1 - x0) / cell_width).round().max(1.0);
        let left_offset = col * spacing;
        let right_offset = (col + span_cells - 1.0) * spacing;
        for vertex in &mut [0, 3] {
            quad.vertices[*vertex].position[0] += left_offset;
        }
        for vertex in &mut [1, 2] {
            quad.vertices[*vertex].position[0] += right_offset;
        }
        let row = (quad.vertices[0].position[1] / quad_height(&quad.vertices))
            .round()
            .max(0.0);
        let y_offset = row * spacing;
        for vertex in &mut quad.vertices {
            vertex.position[1] += y_offset;
        }
    }
}

pub(super) fn apply_background_cell_spacing(batch: &mut BackgroundQuadBatch, cell_spacing_px: u16) {
    if cell_spacing_px == 0 {
        return;
    }
    let spacing = f32::from(cell_spacing_px);
    for quad in &mut batch.quads {
        let left_offset = f32::from(quad.col) * spacing;
        let right_offset =
            f32::from(quad.col.saturating_add(quad.cols).saturating_sub(1)) * spacing;
        let y_offset = f32::from(quad.row) * spacing;
        for vertex in &mut [0, 3] {
            quad.vertices[*vertex].position[0] += left_offset;
        }
        for vertex in &mut [1, 2] {
            quad.vertices[*vertex].position[0] += right_offset;
        }
        for vertex in &mut quad.vertices {
            vertex.position[1] += y_offset;
        }
    }
}

pub(super) fn translate_glyph_batch(batch: &mut GlyphQuadBatch, surface_padding_px: u16) {
    if surface_padding_px == 0 {
        return;
    }
    let offset = f32::from(surface_padding_px);
    for quad in &mut batch.quads {
        for vertex in &mut quad.vertices {
            translate_glyph_vertex(vertex, offset);
        }
    }
}

pub(super) fn translate_background_batch(batch: &mut BackgroundQuadBatch, surface_padding_px: u16) {
    if surface_padding_px == 0 {
        return;
    }
    let offset = f32::from(surface_padding_px);
    for quad in &mut batch.quads {
        for vertex in &mut quad.vertices {
            translate_background_vertex(vertex, offset);
        }
    }
}

fn translate_glyph_vertex(vertex: &mut GlyphVertex, offset: f32) {
    vertex.position[0] += offset;
    vertex.position[1] += offset;
}

fn quad_height(vertices: &[GlyphVertex; 4]) -> f32 {
    (vertices[3].position[1] - vertices[0].position[1]).max(1.0)
}

fn translate_background_vertex(vertex: &mut BackgroundVertex, offset: f32) {
    vertex.position[0] += offset;
    vertex.position[1] += offset;
}
