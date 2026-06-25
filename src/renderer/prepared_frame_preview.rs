use super::{
    BackgroundQuad, BackgroundQuadBatch, GlyphAtlasImage, GlyphQuad, SurfaceFrameError,
    SurfaceGlyphFrame,
};

mod buffer;
mod color;
mod geometry;

use buffer::{blend_pixel, checked_pixel_count, checked_rgba_len, fill_rect, rgba_offset};
use color::{is_grayscale, linear_f32_rgba_to_srgb8, linear_f64_rgba_to_srgb8, multiply_u8};
use geometry::clipped_quad_rect;

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

fn draw_glyph_batch(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    atlas: &GlyphAtlasImage,
    batch: &super::GlyphQuadBatch,
) -> std::result::Result<(), SurfaceFrameError> {
    validate_atlas_rgba(atlas)?;
    for quad in &batch.quads {
        draw_glyph_quad(rgba, width, height, atlas, quad)?;
    }
    Ok(())
}

fn validate_atlas_rgba(atlas: &GlyphAtlasImage) -> std::result::Result<(), SurfaceFrameError> {
    let expected = checked_rgba_len(atlas.width, atlas.height)?;
    if atlas.rgba.len() != expected {
        return Err(SurfaceFrameError::InvalidFrame(format!(
            "prepared frame preview expected {expected} atlas RGBA bytes, got {}",
            atlas.rgba.len()
        )));
    }
    Ok(())
}

fn draw_glyph_quad(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    atlas: &GlyphAtlasImage,
    quad: &GlyphQuad,
) -> std::result::Result<(), SurfaceFrameError> {
    let Some(rect) = clipped_quad_rect(
        quad.vertices.iter().map(|vertex| vertex.position),
        width,
        height,
    ) else {
        return Ok(());
    };
    let x0 = quad.vertices[0].position[0];
    let y0 = quad.vertices[0].position[1];
    let x1 = quad.vertices[2].position[0];
    let y1 = quad.vertices[2].position[1];
    if x1 <= x0 || y1 <= y0 {
        return Ok(());
    }
    let u0 = quad.vertices[0].uv[0];
    let v0 = quad.vertices[0].uv[1];
    let u1 = quad.vertices[2].uv[0];
    let v1 = quad.vertices[2].uv[1];
    let foreground = linear_f32_rgba_to_srgb8(quad.vertices[0].foreground_rgba);

    for y in rect.y0..rect.y1 {
        let ty = ((y as f32 + 0.5) - y0) / (y1 - y0);
        let v = v0 + ((v1 - v0) * ty.clamp(0.0, 1.0));
        let atlas_y = sampled_atlas_coord(v, atlas.height);
        for x in rect.x0..rect.x1 {
            let tx = ((x as f32 + 0.5) - x0) / (x1 - x0);
            let u = u0 + ((u1 - u0) * tx.clamp(0.0, 1.0));
            let atlas_x = sampled_atlas_coord(u, atlas.width);
            let atlas_pixel = atlas_pixel(atlas, atlas_x, atlas_y)?;
            if atlas_pixel[3] == 0 {
                continue;
            }
            let mut glyph = atlas_pixel;
            if is_grayscale(atlas_pixel) {
                glyph[0] = multiply_u8(atlas_pixel[0], foreground[0]);
                glyph[1] = multiply_u8(atlas_pixel[1], foreground[1]);
                glyph[2] = multiply_u8(atlas_pixel[2], foreground[2]);
            }
            glyph[3] = multiply_u8(atlas_pixel[3], foreground[3]);
            blend_pixel(rgba, width, x, y, glyph)?;
        }
    }
    Ok(())
}

fn sampled_atlas_coord(uv: f32, size: u32) -> u32 {
    let max = size.saturating_sub(1);
    ((uv.clamp(0.0, 1.0) * size as f32).floor() as u32).min(max)
}

fn atlas_pixel(
    atlas: &GlyphAtlasImage,
    x: u32,
    y: u32,
) -> std::result::Result<[u8; 4], SurfaceFrameError> {
    let offset = rgba_offset(atlas.width, x, y)?;
    Ok([
        atlas.rgba[offset],
        atlas.rgba[offset + 1],
        atlas.rgba[offset + 2],
        atlas.rgba[offset + 3],
    ])
}
