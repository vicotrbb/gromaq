//! Geometry helpers for owned surface glyph-frame preparation.

use super::{BackgroundQuadBatch, BackgroundVertex, GlyphQuadBatch, GlyphVertex};
use crate::renderer::SurfaceFrameError;

pub(super) fn checked_surface_frame_pixel_dimension(
    label: &'static str,
    cells: u16,
    cell_size_px: u32,
    surface_padding_px: u16,
) -> std::result::Result<u32, SurfaceFrameError> {
    u32::from(cells)
        .checked_mul(cell_size_px)
        .and_then(|cell_pixels| {
            u32::from(surface_padding_px)
                .checked_mul(2)
                .and_then(|padding_pixels| cell_pixels.checked_add(padding_pixels))
        })
        .ok_or_else(|| {
            SurfaceFrameError::InvalidFrame(format!("{label} is too large to represent"))
        })
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

fn translate_background_vertex(vertex: &mut BackgroundVertex, offset: f32) {
    vertex.position[0] += offset;
    vertex.position[1] += offset;
}
