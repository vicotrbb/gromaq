mod background;
mod cursor;
mod glyph;
mod text_decoration;

pub use background::{
    BackgroundQuad, BackgroundQuadBatch, BackgroundQuadConfig, BackgroundQuadError,
    BackgroundQuadPlanner, BackgroundVertex,
};
pub(in crate::renderer::quads) use background::{
    checked_background_quad_base_index, checked_background_quad_index_capacity,
};
pub use cursor::{CursorQuadConfig, CursorQuadPlanner};
pub use glyph::{
    GlyphQuad, GlyphQuadBatch, GlyphQuadConfig, GlyphQuadError, GlyphQuadPlanner, GlyphVertex,
};
pub use text_decoration::{TextDecorationQuadConfig, TextDecorationQuadPlanner};
