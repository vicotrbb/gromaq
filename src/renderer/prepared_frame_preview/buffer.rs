use crate::renderer::SurfaceFrameError;

use super::color::blend_channel;
use super::geometry::PixelRect;

pub(super) fn checked_pixel_count(
    width: u32,
    height: u32,
) -> std::result::Result<usize, SurfaceFrameError> {
    usize::try_from(u64::from(width) * u64::from(height)).map_err(|_| {
        SurfaceFrameError::InvalidFrame("prepared frame preview is too large".to_owned())
    })
}

pub(super) fn checked_rgba_len(
    width: u32,
    height: u32,
) -> std::result::Result<usize, SurfaceFrameError> {
    usize::try_from(u64::from(width) * u64::from(height) * 4).map_err(|_| {
        SurfaceFrameError::InvalidFrame(
            "prepared frame preview RGBA buffer is too large".to_owned(),
        )
    })
}

pub(super) fn fill_rect(
    rgba: &mut [u8],
    width: u32,
    rect: PixelRect,
    color: [u8; 4],
) -> std::result::Result<(), SurfaceFrameError> {
    for y in rect.y0..rect.y1 {
        for x in rect.x0..rect.x1 {
            blend_pixel(rgba, width, x, y, color)?;
        }
    }
    Ok(())
}

pub(super) fn blend_pixel(
    rgba: &mut [u8],
    width: u32,
    x: u32,
    y: u32,
    src: [u8; 4],
) -> std::result::Result<(), SurfaceFrameError> {
    let offset = rgba_offset(width, x, y)?;
    let alpha = f32::from(src[3]) / 255.0;
    let inverse = 1.0 - alpha;
    rgba[offset] = blend_channel(src[0], rgba[offset], alpha, inverse);
    rgba[offset + 1] = blend_channel(src[1], rgba[offset + 1], alpha, inverse);
    rgba[offset + 2] = blend_channel(src[2], rgba[offset + 2], alpha, inverse);
    rgba[offset + 3] = 255;
    Ok(())
}

pub(super) fn rgba_offset(
    width: u32,
    x: u32,
    y: u32,
) -> std::result::Result<usize, SurfaceFrameError> {
    usize::try_from((u64::from(y) * u64::from(width) + u64::from(x)) * 4).map_err(|_| {
        SurfaceFrameError::InvalidFrame(
            "prepared frame preview pixel offset is too large".to_owned(),
        )
    })
}
