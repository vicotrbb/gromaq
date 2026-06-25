use crate::renderer::{GlyphAtlasImage, GlyphQuad, GlyphQuadBatch, SurfaceFrameError};

use super::buffer::{blend_pixel, checked_rgba_len, rgba_offset};
use super::color::{is_grayscale, linear_f32_rgba_to_srgb8, multiply_u8};
use super::geometry::clipped_quad_rect;

pub(super) fn draw_glyph_batch(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    atlas: &GlyphAtlasImage,
    batch: &GlyphQuadBatch,
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
