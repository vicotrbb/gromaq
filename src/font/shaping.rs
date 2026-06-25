use swash::{FontRef, GlyphId};

use super::FontRasterizer;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::font) struct ShapedGlyph {
    pub(in crate::font) id: GlyphId,
    pub(in crate::font) x: f32,
    pub(in crate::font) y: f32,
}

impl FontRasterizer {
    pub(in crate::font) fn shape_text(&mut self, text: &str, size_px: f32) -> Vec<ShapedGlyph> {
        let font = FontRef {
            data: &self.font_bytes,
            offset: self.offset,
            key: self.key,
        };
        let mut shaper = self.shape_context.builder(font).size(size_px).build();
        shaper.add_str(text);

        let mut glyphs = Vec::new();
        let mut pen_x = 0.0;
        shaper.shape_with(|cluster| {
            for glyph in cluster.glyphs {
                glyphs.push(ShapedGlyph {
                    id: glyph.id,
                    x: pen_x + glyph.x,
                    y: glyph.y,
                });
                pen_x += glyph.advance;
            }
        });
        glyphs
    }
}
