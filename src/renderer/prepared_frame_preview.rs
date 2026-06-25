use super::{BackgroundQuad, BackgroundQuadBatch, SurfaceFrameError, SurfaceGlyphFrame};

mod buffer;
mod color;
mod geometry;
mod glyph;

use buffer::{checked_pixel_count, checked_rgba_len, fill_rect};
use color::{linear_f32_rgba_to_srgb8, linear_f64_rgba_to_srgb8};
use geometry::clipped_quad_rect;
use glyph::draw_glyph_batch;

/// Deterministic CPU preview of a prepared surface glyph frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedFramePreview {
    /// Preview width in pixels.
    pub width: u32,
    /// Preview height in pixels.
    pub height: u32,
    /// RGBA8 pixels in row-major order.
    pub rgba: Vec<u8>,
}

pub(super) fn preview_surface_glyph_frame(
    frame: SurfaceGlyphFrame<'_>,
) -> std::result::Result<PreparedFramePreview, SurfaceFrameError> {
    let pixel_count = checked_pixel_count(frame.width, frame.height)?;
    let byte_len = checked_rgba_len(frame.width, frame.height)?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(byte_len).map_err(|_| {
        SurfaceFrameError::InvalidFrame("prepared frame preview allocation failed".to_owned())
    })?;
    let clear = linear_f64_rgba_to_srgb8(frame.clear_color);
    for _ in 0..pixel_count {
        rgba.extend_from_slice(&clear);
    }

    draw_background_batch(&mut rgba, frame.width, frame.height, frame.background_batch)?;
    draw_glyph_batch(
        &mut rgba,
        frame.width,
        frame.height,
        frame.atlas,
        frame.batch,
    )?;
    draw_background_batch(&mut rgba, frame.width, frame.height, frame.decoration_batch)?;
    draw_background_batch(&mut rgba, frame.width, frame.height, frame.cursor_batch)?;

    Ok(PreparedFramePreview {
        width: frame.width,
        height: frame.height,
        rgba,
    })
}

fn draw_background_batch(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    batch: &BackgroundQuadBatch,
) -> std::result::Result<(), SurfaceFrameError> {
    for quad in &batch.quads {
        draw_solid_quad(rgba, width, height, quad)?;
    }
    Ok(())
}

fn draw_solid_quad(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    quad: &BackgroundQuad,
) -> std::result::Result<(), SurfaceFrameError> {
    let Some(rect) = clipped_quad_rect(
        quad.vertices.iter().map(|vertex| vertex.position),
        width,
        height,
    ) else {
        return Ok(());
    };
    let color = linear_f32_rgba_to_srgb8(quad.vertices[0].color_rgba);
    fill_rect(rgba, width, rect, color)
}
