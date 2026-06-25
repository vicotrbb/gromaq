//! CPU-side render planning from terminal snapshots.

use crate::config::{DEFAULT_ANSI_COLORS_RGB8, DEFAULT_DIM_OPACITY, DEFAULT_SELECTION_RGB8};
use crate::dirty::DirtyRegion;
use crate::error::Result;
use crate::grid::GridSnapshot;
use crate::selection::SelectionRange;
use crate::terminal::CursorSnapshot;

use super::atlas::{GlyphAtlas, GlyphKey};
use super::color::style_background_rgba8;
use clipping::clipped_dirty_region;
use decorations::append_cell_decorations;
pub use types::{
    PlannedBackground, PlannedGlyph, PlannedTextDecoration, RenderPlan, TextDecorationKind,
};

mod clipping;
mod decorations;
#[cfg(test)]
mod tests;
mod types;

/// CPU-side render planner for deterministic renderer tests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderPlanner {
    font_size_px: u16,
    default_foreground_rgb8: [u8; 3],
    ansi_colors_rgb8: [[u8; 3]; 16],
    selection_background_rgba8: [u8; 4],
    dim_opacity: f32,
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
        Self::with_visual_theme(
            font_size_px,
            default_foreground_rgb8,
            ansi_colors_rgb8,
            [
                DEFAULT_SELECTION_RGB8[0],
                DEFAULT_SELECTION_RGB8[1],
                DEFAULT_SELECTION_RGB8[2],
                255,
            ],
            DEFAULT_DIM_OPACITY,
        )
    }

    /// Create a render planner for a fixed font size and full visual theme.
    pub fn with_visual_theme(
        font_size_px: u16,
        default_foreground_rgb8: [u8; 3],
        ansi_colors_rgb8: [[u8; 3]; 16],
        selection_background_rgba8: [u8; 4],
        dim_opacity: f32,
    ) -> Self {
        Self {
            font_size_px,
            default_foreground_rgb8,
            ansi_colors_rgb8,
            selection_background_rgba8,
            dim_opacity,
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
                    let color_rgba8 = if is_selected(grid.selection, row, col) {
                        Some(self.selection_background_rgba8)
                    } else {
                        style_background_rgba8(
                            cell.style,
                            self.default_foreground_rgb8,
                            self.ansi_colors_rgb8,
                        )
                    };
                    if let Some(color_rgba8) = color_rgba8 {
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
            dim_opacity: self.dim_opacity,
            clear_regions: dirty_regions.to_vec(),
            backgrounds,
            decorations,
            glyphs,
        })
    }
}

fn is_selected(selection: Option<SelectionRange>, row: u16, col: u16) -> bool {
    let Some(selection) = selection else {
        return false;
    };
    row >= selection.start.row
        && row <= selection.end.row
        && col >= selection_start_col(selection, row)
        && col <= selection_end_col(selection, row)
}

fn selection_start_col(selection: SelectionRange, row: u16) -> u16 {
    if row == selection.start.row {
        selection.start.col
    } else {
        0
    }
}

fn selection_end_col(selection: SelectionRange, row: u16) -> u16 {
    if row == selection.end.row {
        selection.end.col
    } else {
        u16::MAX
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
