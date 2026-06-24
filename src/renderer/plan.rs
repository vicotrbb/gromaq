//! CPU-side render planning from terminal snapshots.

use crate::cell::{Color, Style, UnderlineStyle};
use crate::dirty::DirtyRegion;
use crate::error::Result;
use crate::grid::GridSnapshot;
use crate::terminal::CursorSnapshot;

use super::atlas::{GlyphAtlas, GlyphEntry, GlyphKey};
use super::color::{decoration_color_rgba8, style_background_rgba8};

/// CPU-side render planner for deterministic renderer tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderPlanner {
    font_size_px: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClippedDirtyRegion {
    row_start: u16,
    row_end: u16,
    col_start: u16,
    col_end: u16,
}

impl ClippedDirtyRegion {
    fn rows(self) -> u16 {
        self.row_end - self.row_start
    }

    fn cols(self) -> u16 {
        self.col_end - self.col_start
    }
}

impl RenderPlanner {
    /// Create a render planner for a fixed font size.
    pub fn new(font_size_px: u16) -> Self {
        Self { font_size_px }
    }

    /// Build a deterministic render plan from a terminal snapshot and dirty regions.
    pub fn plan_frame(
        &mut self,
        grid: &GridSnapshot,
        cursor: CursorSnapshot,
        dirty_regions: &[DirtyRegion],
        atlas: &mut GlyphAtlas,
    ) -> Result<RenderPlan> {
        let estimated_dirty_cells = dirty_regions
            .iter()
            .filter_map(|region| clipped_dirty_region(region, grid))
            .map(|region| usize::from(region.rows()) * usize::from(region.cols()))
            .sum();
        let mut glyphs = Vec::with_capacity(estimated_dirty_cells);
        let mut backgrounds = Vec::new();
        let mut decorations = Vec::new();
        for region in dirty_regions {
            let Some(region) = clipped_dirty_region(region, grid) else {
                continue;
            };
            for row in region.row_start..region.row_end {
                for col in region.col_start..region.col_end {
                    let cell = grid.cell(row, col);
                    if let Some(color_rgba8) = style_background_rgba8(cell.style) {
                        append_background_fill(&mut backgrounds, row, col, color_rgba8);
                    }
                    append_cell_decorations(
                        &mut decorations,
                        row,
                        col,
                        cell.style,
                        grid.cell_underline_color(row, col),
                    );
                    if cell.text.is_empty() || cell.is_wide_trailing {
                        continue;
                    }
                    if cell.text.chars().all(char::is_whitespace) {
                        continue;
                    }
                    let Some(ch) = cell.text.chars().next() else {
                        continue;
                    };
                    let text = cell.text.clone();
                    let glyph_key = GlyphKey::for_text(&text, ch, cell.style, self.font_size_px);
                    let atlas_entry = atlas.lookup_or_insert(glyph_key)?;
                    glyphs.push(PlannedGlyph {
                        row,
                        col,
                        text,
                        ch,
                        style: cell.style,
                        font_size_px: self.font_size_px,
                        is_wide: cell.is_wide_leading,
                        atlas_entry,
                    });
                }
            }
        }
        Ok(RenderPlan {
            viewport_cols: grid.cols,
            viewport_rows: grid.rows,
            cursor,
            clear_regions: dirty_regions.to_vec(),
            backgrounds,
            decorations,
            glyphs,
        })
    }
}

fn append_background_fill(
    backgrounds: &mut Vec<PlannedBackground>,
    row: u16,
    col: u16,
    color_rgba8: [u8; 4],
) {
    if let Some(last) = backgrounds.last_mut()
        && last.row == row
        && last.col.saturating_add(last.cols) == col
        && last.color_rgba8 == color_rgba8
    {
        last.cols = last.cols.saturating_add(1);
        return;
    }
    backgrounds.push(PlannedBackground {
        row,
        col,
        cols: 1,
        color_rgba8,
    });
}

fn append_cell_decorations(
    decorations: &mut Vec<PlannedTextDecoration>,
    row: u16,
    col: u16,
    style: Style,
    underline_color: Color,
) {
    if style.hidden {
        return;
    }
    if style.underline {
        match style.underline_style {
            UnderlineStyle::Single => append_text_decoration(
                decorations,
                row,
                col,
                TextDecorationKind::Underline,
                decoration_color_rgba8(underline_color, style),
            ),
            UnderlineStyle::Double => {
                let color_rgba8 = decoration_color_rgba8(underline_color, style);
                append_text_decoration(
                    decorations,
                    row,
                    col,
                    TextDecorationKind::DoubleUnderlineTop,
                    color_rgba8,
                );
                append_text_decoration(
                    decorations,
                    row,
                    col,
                    TextDecorationKind::DoubleUnderlineBottom,
                    color_rgba8,
                );
            }
            UnderlineStyle::Curly => append_text_decoration(
                decorations,
                row,
                col,
                TextDecorationKind::CurlyUnderline,
                decoration_color_rgba8(underline_color, style),
            ),
            UnderlineStyle::Dotted => append_text_decoration(
                decorations,
                row,
                col,
                TextDecorationKind::DottedUnderline,
                decoration_color_rgba8(underline_color, style),
            ),
            UnderlineStyle::Dashed => append_text_decoration(
                decorations,
                row,
                col,
                TextDecorationKind::DashedUnderline,
                decoration_color_rgba8(underline_color, style),
            ),
        }
    }
    if style.overline {
        append_text_decoration(
            decorations,
            row,
            col,
            TextDecorationKind::Overline,
            decoration_color_rgba8(Color::Default, style),
        );
    }
    if style.strikethrough {
        append_text_decoration(
            decorations,
            row,
            col,
            TextDecorationKind::Strikethrough,
            decoration_color_rgba8(Color::Default, style),
        );
    }
}

fn append_text_decoration(
    decorations: &mut Vec<PlannedTextDecoration>,
    row: u16,
    col: u16,
    kind: TextDecorationKind,
    color_rgba8: [u8; 4],
) {
    if let Some(last) = decorations.iter_mut().rev().take(4).find(|last| {
        last.row == row
            && last.col.saturating_add(last.cols) == col
            && last.kind == kind
            && last.color_rgba8 == color_rgba8
    }) {
        last.cols = last.cols.saturating_add(1);
        return;
    }
    decorations.push(PlannedTextDecoration {
        row,
        col,
        cols: 1,
        kind,
        color_rgba8,
    });
}

fn clipped_dirty_region(region: &DirtyRegion, grid: &GridSnapshot) -> Option<ClippedDirtyRegion> {
    let row_start = region.row.min(grid.rows);
    let col_start = region.col.min(grid.cols);
    let row_end = (u32::from(region.row) + u32::from(region.rows)).min(u32::from(grid.rows));
    let col_end = (u32::from(region.col) + u32::from(region.cols)).min(u32::from(grid.cols));
    let row_end = u16::try_from(row_end).ok()?;
    let col_end = u16::try_from(col_end).ok()?;
    if row_start >= row_end || col_start >= col_end {
        return None;
    }
    Some(ClippedDirtyRegion {
        row_start,
        row_end,
        col_start,
        col_end,
    })
}

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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::terminal::{Terminal, TerminalConfig};

    fn empty_grid_snapshot(rows: u16, cols: u16) -> GridSnapshot {
        GridSnapshot {
            rows,
            cols,
            hyperlinks: Vec::new(),
            underline_colors: Vec::new(),
            cells: Vec::new(),
        }
    }

    #[test]
    fn clipped_dirty_region_uses_widened_bounds_at_u16_edges() {
        let grid = empty_grid_snapshot(u16::MAX, u16::MAX);
        let region = DirtyRegion {
            row: u16::MAX - 1,
            col: u16::MAX - 2,
            rows: 8,
            cols: 9,
        };

        assert_eq!(
            clipped_dirty_region(&region, &grid),
            Some(ClippedDirtyRegion {
                row_start: u16::MAX - 1,
                row_end: u16::MAX,
                col_start: u16::MAX - 2,
                col_end: u16::MAX,
            })
        );
    }

    #[test]
    fn clipped_dirty_region_rejects_regions_outside_grid() {
        let grid = empty_grid_snapshot(10, 10);
        let region = DirtyRegion {
            row: 12,
            col: 0,
            rows: 1,
            cols: 1,
        };

        assert_eq!(clipped_dirty_region(&region, &grid), None);
    }

    #[test]
    fn render_planner_ignores_dirty_regions_outside_grid() {
        let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());
        terminal.write_str("AB").unwrap();
        let mut atlas = GlyphAtlas::new(super::super::GlyphAtlasConfig::new(8).unwrap());
        let mut planner = RenderPlanner::new(14);
        let dirty = [DirtyRegion {
            row: 4,
            col: 0,
            rows: 1,
            cols: 1,
        }];

        let plan = planner
            .plan_frame(
                &terminal.dump_grid(),
                terminal.dump_cursor(),
                &dirty,
                &mut atlas,
            )
            .unwrap();

        assert!(plan.glyphs.is_empty());
        assert_eq!(atlas.metrics().entries, 0);
    }
}
