use crate::renderer::{GlyphBitmap, GlyphEntry};

use super::{rgba_offset, zeroed_rgba_buffer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::font) struct RenderedGlyph {
    pub(in crate::font) x: i32,
    pub(in crate::font) y: i32,
    pub(in crate::font) width: u32,
    pub(in crate::font) height: u32,
    pub(in crate::font) rgba: Vec<u8>,
}

pub(in crate::font) fn compose_rendered_glyphs(
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

#[cfg(test)]
mod tests {
    use super::*;

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
