//! Render-plan command data passed to the GPU renderer boundary.

use crate::cell::Style;
use crate::dirty::DirtyRegion;
use crate::terminal::CursorSnapshot;

use crate::renderer::atlas::GlyphEntry;

/// Deterministic CPU-side frame plan consumed by the native renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPlan {
    /// Viewport columns represented by this plan.
    pub viewport_cols: u16,
    /// Viewport rows represented by this plan.
    pub viewport_rows: u16,
    /// Cursor state to draw for this frame.
    pub cursor: CursorSnapshot,
    /// Dirty rectangles to clear before drawing glyphs.
    pub clear_regions: Vec<DirtyRegion>,
    /// Styled cell background fills in row-major order.
    pub backgrounds: Vec<PlannedBackground>,
    /// Styled text-decoration fills in row-major order.
    pub decorations: Vec<PlannedTextDecoration>,
    /// Glyph draw commands in row-major order.
    pub glyphs: Vec<PlannedGlyph>,
}

/// One solid background fill command inside a render plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedBackground {
    /// Grid row.
    pub row: u16,
    /// Starting grid column.
    pub col: u16,
    /// Number of adjacent cells covered by this fill.
    pub cols: u16,
    /// Background color in RGBA8.
    pub color_rgba8: [u8; 4],
}

/// Text-decoration line kind inside a render plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDecorationKind {
    /// Single straight underline.
    Underline,
    /// Upper line of a double straight underline.
    DoubleUnderlineTop,
    /// Lower line of a double straight underline.
    DoubleUnderlineBottom,
    /// Curly underline.
    CurlyUnderline,
    /// Dotted underline.
    DottedUnderline,
    /// Dashed underline.
    DashedUnderline,
    /// Straight overline.
    Overline,
    /// Straight strikethrough.
    Strikethrough,
}

/// One solid text-decoration fill command inside a render plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedTextDecoration {
    /// Grid row.
    pub row: u16,
    /// Starting grid column.
    pub col: u16,
    /// Number of adjacent cells covered by this decoration fill.
    pub cols: u16,
    /// Decoration line kind.
    pub kind: TextDecorationKind,
    /// Decoration color in RGBA8.
    pub color_rgba8: [u8; 4],
}

/// One glyph draw command inside a render plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedGlyph {
    /// Grid row.
    pub row: u16,
    /// Grid column.
    pub col: u16,
    /// Full terminal cell text to draw.
    pub text: String,
    /// Character to draw.
    pub ch: char,
    /// Cell style for the glyph.
    pub style: Style,
    /// Font size used when allocating the glyph atlas entry.
    pub font_size_px: u16,
    /// Whether this glyph occupies two terminal cells.
    pub is_wide: bool,
    /// Glyph atlas handle allocated for this glyph.
    pub atlas_entry: GlyphEntry,
}
