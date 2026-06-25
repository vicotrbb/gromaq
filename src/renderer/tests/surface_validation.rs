use crate::renderer::{
    BackgroundQuad, BackgroundQuadBatch, BackgroundVertex, GlyphEntry, GlyphQuad, GlyphQuadBatch,
    GlyphVertex,
};

mod buffers;
mod frame;

fn one_quad_batch() -> GlyphQuadBatch {
    let quad = GlyphQuad {
        text: "A".to_owned(),
        ch: 'A',
        atlas_entry: GlyphEntry {
            slot: 0,
            generation: 0,
        },
        vertices: [
            GlyphVertex {
                position: [0.0, 0.0],
                uv: [0.0, 0.0],
                foreground_rgba: [1.0, 1.0, 1.0, 1.0],
            },
            GlyphVertex {
                position: [1.0, 0.0],
                uv: [1.0, 0.0],
                foreground_rgba: [1.0, 1.0, 1.0, 1.0],
            },
            GlyphVertex {
                position: [1.0, 1.0],
                uv: [1.0, 1.0],
                foreground_rgba: [1.0, 1.0, 1.0, 1.0],
            },
            GlyphVertex {
                position: [0.0, 1.0],
                uv: [0.0, 1.0],
                foreground_rgba: [1.0, 1.0, 1.0, 1.0],
            },
        ],
    };
    GlyphQuadBatch {
        quads: vec![quad],
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}

fn one_background_batch(color_rgba: [f32; 4]) -> BackgroundQuadBatch {
    BackgroundQuadBatch {
        quads: vec![BackgroundQuad {
            row: 0,
            col: 0,
            cols: 1,
            vertices: [
                BackgroundVertex {
                    position: [0.0, 0.0],
                    color_rgba,
                },
                BackgroundVertex {
                    position: [1.0, 0.0],
                    color_rgba,
                },
                BackgroundVertex {
                    position: [1.0, 1.0],
                    color_rgba,
                },
                BackgroundVertex {
                    position: [0.0, 1.0],
                    color_rgba,
                },
            ],
        }],
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}
