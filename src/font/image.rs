use swash::scale::image::Content;

use crate::renderer::{GlyphBitmap, GlyphEntry};

use super::FontRasterError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RenderedGlyph {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) rgba: Vec<u8>,
}

pub(super) fn compose_rendered_glyphs(
    entry: GlyphEntry,
    glyphs: &[RenderedGlyph],
) -> Option<GlyphBitmap> {
    let first = glyphs.first()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = checked_glyph_edge(first.x, first.width)?;
    let mut max_y = checked_glyph_edge(first.y, first.height)?;

    for glyph in &glyphs[1..] {
        min_x = min_x.min(glyph.x);
        min_y = min_y.min(glyph.y);
        max_x = max_x.max(checked_glyph_edge(glyph.x, glyph.width)?);
        max_y = max_y.max(checked_glyph_edge(glyph.y, glyph.height)?);
    }

    let width = checked_glyph_span(min_x, max_x)?;
    let height = checked_glyph_span(min_y, max_y)?;
    if width == 0 || height == 0 {
        return None;
    }

    let mut rgba = zeroed_rgba_buffer(width, height)?;
    for glyph in glyphs {
        blend_glyph_into_canvas(glyph, min_x, min_y, width, &mut rgba)?;
    }

    Some(GlyphBitmap {
        entry,
        origin_x: min_x,
        origin_y: min_y,
        width,
        height,
        rgba,
    })
}

fn checked_glyph_edge(origin: i32, extent: u32) -> Option<i32> {
    origin.checked_add(i32::try_from(extent).ok()?)
}

fn checked_glyph_span(min: i32, max: i32) -> Option<u32> {
    u32::try_from(max.checked_sub(min)?).ok()
}

fn checked_glyph_canvas_offset(position: i32, origin: i32) -> Option<u32> {
    u32::try_from(position.checked_sub(origin)?).ok()
}

fn blend_glyph_into_canvas(
    glyph: &RenderedGlyph,
    min_x: i32,
    min_y: i32,
    canvas_width: u32,
    canvas: &mut [u8],
) -> Option<()> {
    let offset_x = checked_glyph_canvas_offset(glyph.x, min_x)?;
    let offset_y = checked_glyph_canvas_offset(glyph.y, min_y)?;
    for source_y in 0..glyph.height {
        for source_x in 0..glyph.width {
            let Some(source_index) = rgba_offset(glyph.width, source_x, source_y) else {
                continue;
            };
            let Some(target_x) = offset_x.checked_add(source_x) else {
                continue;
            };
            let Some(target_y) = offset_y.checked_add(source_y) else {
                continue;
            };
            let Some(target_index) = rgba_offset(canvas_width, target_x, target_y) else {
                continue;
            };
            let source_end = source_index.checked_add(4)?;
            let target_end = target_index.checked_add(4)?;
            if source_end > glyph.rgba.len() || target_end > canvas.len() {
                return None;
            }
            let source_alpha = glyph.rgba[source_index + 3];
            if source_alpha > canvas[target_index + 3] {
                canvas[target_index..target_end]
                    .copy_from_slice(&glyph.rgba[source_index..source_end]);
            }
        }
    }
    Some(())
}

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
    fn glyph_geometry_helpers_reject_overflowing_bounds() {
        assert_eq!(checked_glyph_edge(-2, 4), Some(2));
        assert_eq!(checked_glyph_edge(i32::MAX, 1), None);
        assert_eq!(checked_glyph_span(-2, 2), Some(4));
        assert_eq!(checked_glyph_span(2, -2), None);
        assert_eq!(checked_glyph_canvas_offset(5, 2), Some(3));
        assert_eq!(checked_glyph_canvas_offset(2, 5), None);
    }

    #[test]
    fn compose_rendered_glyphs_rejects_overflowing_bounds() {
        let glyph = RenderedGlyph {
            x: i32::MAX,
            y: 0,
            width: 1,
            height: 1,
            rgba: vec![255, 255, 255, 255],
        };

        assert_eq!(
            compose_rendered_glyphs(
                GlyphEntry {
                    slot: 0,
                    generation: 0,
                },
                &[glyph],
            ),
            None
        );
    }

    #[test]
    fn compose_rendered_glyphs_rejects_truncated_source_rgba() {
        let glyph = RenderedGlyph {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            rgba: vec![255, 255, 255],
        };

        assert_eq!(
            compose_rendered_glyphs(
                GlyphEntry {
                    slot: 0,
                    generation: 0,
                },
                &[glyph],
            ),
            None
        );
    }
}
