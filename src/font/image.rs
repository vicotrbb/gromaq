use swash::scale::image::Content;

use super::FontRasterError;

mod composition;
pub(super) use composition::{RenderedGlyph, compose_rendered_glyphs};

fn rgba_pixel_count(width: u32, height: u32) -> Option<usize> {
    usize::try_from(width).ok().and_then(|width| {
        usize::try_from(height)
            .ok()
            .and_then(|height| width.checked_mul(height))
    })
}

fn rgba_byte_len(width: u32, height: u32) -> Option<usize> {
    rgba_pixel_count(width, height).and_then(|pixels| pixels.checked_mul(4))
}

fn zeroed_rgba_buffer(width: u32, height: u32) -> Option<Vec<u8>> {
    let len = rgba_byte_len(width, height)?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(len).ok()?;
    rgba.resize(len, 0);
    Some(rgba)
}

fn rgba_offset(width: u32, x: u32, y: u32) -> Option<usize> {
    usize::try_from(y)
        .ok()
        .and_then(|y| {
            usize::try_from(width)
                .ok()
                .and_then(|width| y.checked_mul(width))
        })
        .and_then(|row_start| {
            usize::try_from(x)
                .ok()
                .and_then(|x| row_start.checked_add(x))
        })
        .and_then(|pixel_offset| pixel_offset.checked_mul(4))
}

pub(super) fn image_to_rgba8(
    content: Content,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<Vec<u8>, FontRasterError> {
    let pixel_count =
        rgba_pixel_count(width, height).ok_or(FontRasterError::InvalidImageBuffer {
            width,
            height,
            content,
        })?;
    match content {
        Content::Mask => {
            if data.len() != pixel_count {
                return Err(FontRasterError::InvalidImageBuffer {
                    width,
                    height,
                    content,
                });
            }
            let expected_len =
                rgba_byte_len(width, height).ok_or(FontRasterError::InvalidImageBuffer {
                    width,
                    height,
                    content,
                })?;
            let mut rgba = Vec::new();
            rgba.try_reserve_exact(expected_len).map_err(|_| {
                FontRasterError::InvalidImageBuffer {
                    width,
                    height,
                    content,
                }
            })?;
            for alpha in data {
                rgba.extend_from_slice(&[255, 255, 255, *alpha]);
            }
            Ok(rgba)
        }
        Content::SubpixelMask | Content::Color => {
            let expected_len =
                rgba_byte_len(width, height).ok_or(FontRasterError::InvalidImageBuffer {
                    width,
                    height,
                    content,
                })?;
            if data.len() != expected_len {
                return Err(FontRasterError::InvalidImageBuffer {
                    width,
                    height,
                    content,
                });
            }
            Ok(data.to_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_to_rgba8_rejects_oversized_mask_dimensions_before_allocation() {
        let error = image_to_rgba8(Content::Mask, u32::MAX, u32::MAX, &[]).unwrap_err();

        assert_eq!(
            error,
            FontRasterError::InvalidImageBuffer {
                width: u32::MAX,
                height: u32::MAX,
                content: Content::Mask,
            }
        );
    }

    #[test]
    fn image_to_rgba8_rejects_oversized_color_dimensions_before_allocation() {
        let error = image_to_rgba8(Content::Color, u32::MAX, u32::MAX, &[]).unwrap_err();

        assert_eq!(
            error,
            FontRasterError::InvalidImageBuffer {
                width: u32::MAX,
                height: u32::MAX,
                content: Content::Color,
            }
        );
    }

    #[test]
    fn image_to_rgba8_accepts_mask_alpha_bytes() {
        assert_eq!(
            image_to_rgba8(Content::Mask, 2, 1, &[0, 255]).unwrap(),
            vec![255, 255, 255, 0, 255, 255, 255, 255]
        );
    }
}
