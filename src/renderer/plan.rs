//! CPU-side render planning from terminal snapshots.

use crate::cell::{Color, Style, UnderlineStyle};
use crate::config::DEFAULT_ANSI_COLORS_RGB8;
use crate::dirty::DirtyRegion;
use crate::error::Result;
use crate::grid::GridSnapshot;
use crate::terminal::CursorSnapshot;

use super::atlas::{GlyphAtlas, GlyphKey};
use super::color::{decoration_color_rgba8, style_background_rgba8};
use clipping::clipped_dirty_region;
pub use types::{
    PlannedBackground, PlannedGlyph, PlannedTextDecoration, RenderPlan, TextDecorationKind,
};

mod clipping;
mod types;

/// CPU-side render planner for deterministic renderer tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderPlanner {
    font_size_px: u16,
    default_foreground_rgb8: [u8; 3],
    ansi_colors_rgb8: [[u8; 3]; 16],
}

impl RenderPlanner {
    /// Create a render planner for a fixed font size.
    pub fn new(font_size_px: u16) -> Self {
        Self::with_default_foreground(font_size_px, [229, 229, 229])
    }

    /// Create a render planner for a fixed font size and default foreground color.
    pub fn with_default_foreground(font_size_px: u16, default_foreground_rgb8: [u8; 3]) -> Self {
        Self::with_theme(
            font_size_px,
            default_foreground_rgb8,
            DEFAULT_ANSI_COLORS_RGB8,
        )
    }

    /// Create a render planner for a fixed font size and theme palette.
    pub fn with_theme(
        font_size_px: u16,
        default_foreground_rgb8: [u8; 3],
        ansi_colors_rgb8: [[u8; 3]; 16],
    ) -> Self {
        Self {
            font_size_px,
            default_foreground_rgb8,
            ansi_colors_rgb8,
        }
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
                    if let Some(color_rgba8) = style_background_rgba8(
                        cell.style,
                        self.default_foreground_rgb8,
                        self.ansi_colors_rgb8,
                    ) {
                        append_background_fill(&mut backgrounds, row, col, color_rgba8);
                    }
                    append_cell_decorations(
                        &mut decorations,
                        row,
                        col,
                        cell.style,
                        grid.cell_underline_color(row, col),
                        self.default_foreground_rgb8,
                        self.ansi_colors_rgb8,
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
            default_foreground_rgb8: self.default_foreground_rgb8,
            ansi_colors_rgb8: self.ansi_colors_rgb8,
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
    default_foreground_rgb8: [u8; 3],
    ansi_colors_rgb8: [[u8; 3]; 16],
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
                decoration_color_rgba8(
                    underline_color,
                    style,
                    default_foreground_rgb8,
                    ansi_colors_rgb8,
                ),
            ),
            UnderlineStyle::Double => {
                let color_rgba8 = decoration_color_rgba8(
                    underline_color,
                    style,
                    default_foreground_rgb8,
                    ansi_colors_rgb8,
                );
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
                decoration_color_rgba8(
                    underline_color,
                    style,
                    default_foreground_rgb8,
                    ansi_colors_rgb8,
                ),
            ),
            UnderlineStyle::Dotted => append_text_decoration(
                decorations,
                row,
                col,
                TextDecorationKind::DottedUnderline,
                decoration_color_rgba8(
                    underline_color,
                    style,
                    default_foreground_rgb8,
                    ansi_colors_rgb8,
                ),
            ),
            UnderlineStyle::Dashed => append_text_decoration(
                decorations,
                row,
                col,
                TextDecorationKind::DashedUnderline,
                decoration_color_rgba8(
                    underline_color,
                    style,
                    default_foreground_rgb8,
                    ansi_colors_rgb8,
                ),
            ),
        }
    }
    if style.overline {
        append_text_decoration(
            decorations,
            row,
            col,
            TextDecorationKind::Overline,
            decoration_color_rgba8(
                Color::Default,
                style,
                default_foreground_rgb8,
                ansi_colors_rgb8,
            ),
        );
    }
    if style.strikethrough {
        append_text_decoration(
            decorations,
            row,
            col,
            TextDecorationKind::Strikethrough,
            decoration_color_rgba8(
                Color::Default,
                style,
                default_foreground_rgb8,
                ansi_colors_rgb8,
            ),
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::terminal::{Terminal, TerminalConfig};

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
