use thiserror::Error;

use crate::terminal::{CursorShape, CursorSnapshot};

use super::atlas::GlyphEntry;
use super::color::{rgba8_to_normalized, style_foreground_rgba};
use super::{
    PlannedBackground, PlannedGlyph, PlannedTextDecoration, RenderPlan, TextDecorationKind,
};

/// Pixel layout used to build solid background quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundQuadConfig {
    /// Terminal cell width in pixels.
    pub cell_width_px: u32,
    /// Terminal cell height in pixels.
    pub cell_height_px: u32,
}

/// Errors produced while building solid background quads.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackgroundQuadError {
    /// Pixel dimensions must be non-zero.
    #[error("background quad dimensions must be non-zero")]
    ZeroDimension,
    /// The planned background batch cannot be represented in `u32` GPU indices.
    #[error("background quad count is too large for u32 GPU indices")]
    IndexCountTooLarge,
}

/// One vertex for a solid background quad.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BackgroundVertex {
    /// Pixel-space output position.
    pub position: [f32; 2],
    /// Solid background color in normalized RGBA.
    pub color_rgba: [f32; 4],
}

/// One solid background quad derived from styled terminal cells.
#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundQuad {
    /// Grid row represented by this quad.
    pub row: u16,
    /// Starting grid column represented by this quad.
    pub col: u16,
    /// Number of adjacent cells represented by this quad.
    pub cols: u16,
    /// Quad vertices in top-left, top-right, bottom-right, bottom-left order.
    pub vertices: [BackgroundVertex; 4],
}

/// Indexed solid background quad batch ready for GPU vertex/index buffer upload.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BackgroundQuadBatch {
    /// Solid background quads.
    pub quads: Vec<BackgroundQuad>,
    /// Triangle indices for all quads.
    pub indices: Vec<u32>,
}

/// Deterministic CPU-side planner for terminal background fill quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundQuadPlanner {
    config: BackgroundQuadConfig,
}

impl BackgroundQuadPlanner {
    /// Create a background quad planner.
    pub fn new(config: BackgroundQuadConfig) -> Self {
        Self { config }
    }

    /// Build solid background quads and triangle indices from a render plan.
    pub fn plan(
        &self,
        plan: &RenderPlan,
    ) -> std::result::Result<BackgroundQuadBatch, BackgroundQuadError> {
        self.validate_config()?;
        let mut quads = Vec::new();
        quads
            .try_reserve_exact(plan.backgrounds.len())
            .map_err(|_| BackgroundQuadError::IndexCountTooLarge)?;
        let mut indices = Vec::new();
        indices
            .try_reserve_exact(checked_background_quad_index_capacity(
                plan.backgrounds.len(),
            )?)
            .map_err(|_| BackgroundQuadError::IndexCountTooLarge)?;

        for background in &plan.backgrounds {
            let quad = self.plan_background(*background);
            let base = checked_background_quad_base_index(quads.len())?;
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            quads.push(quad);
        }

        Ok(BackgroundQuadBatch { quads, indices })
    }

    fn validate_config(&self) -> std::result::Result<(), BackgroundQuadError> {
        if self.config.cell_width_px == 0 || self.config.cell_height_px == 0 {
            return Err(BackgroundQuadError::ZeroDimension);
        }
        Ok(())
    }

    fn plan_background(&self, background: PlannedBackground) -> BackgroundQuad {
        let cell_width = self.config.cell_width_px as f32;
        let cell_height = self.config.cell_height_px as f32;
        let x0 = f32::from(background.col) * cell_width;
        let y0 = f32::from(background.row) * cell_height;
        let x1 = x0 + (cell_width * f32::from(background.cols));
        let y1 = y0 + cell_height;
        let color_rgba = rgba8_to_normalized(background.color_rgba8);

        BackgroundQuad {
            row: background.row,
            col: background.col,
            cols: background.cols,
            vertices: [
                BackgroundVertex {
                    position: [x0, y0],
                    color_rgba,
                },
                BackgroundVertex {
                    position: [x1, y0],
                    color_rgba,
                },
                BackgroundVertex {
                    position: [x1, y1],
                    color_rgba,
                },
                BackgroundVertex {
                    position: [x0, y1],
                    color_rgba,
                },
            ],
        }
    }
}

/// Pixel layout used to build solid text-decoration quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextDecorationQuadConfig {
    /// Terminal cell width in pixels.
    pub cell_width_px: u32,
    /// Terminal cell height in pixels.
    pub cell_height_px: u32,
}

/// Deterministic CPU-side planner for straight terminal text-decoration quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextDecorationQuadPlanner {
    config: TextDecorationQuadConfig,
}

impl TextDecorationQuadPlanner {
    /// Create a text-decoration quad planner.
    pub fn new(config: TextDecorationQuadConfig) -> Self {
        Self { config }
    }

    /// Build solid decoration quads and triangle indices from a render plan.
    pub fn plan(
        &self,
        plan: &RenderPlan,
    ) -> std::result::Result<BackgroundQuadBatch, BackgroundQuadError> {
        self.validate_config()?;
        let mut quads = Vec::new();
        quads
            .try_reserve_exact(plan.decorations.len())
            .map_err(|_| BackgroundQuadError::IndexCountTooLarge)?;
        let mut indices = Vec::new();
        indices
            .try_reserve_exact(checked_background_quad_index_capacity(
                plan.decorations.len(),
            )?)
            .map_err(|_| BackgroundQuadError::IndexCountTooLarge)?;

        for decoration in &plan.decorations {
            self.append_decoration(*decoration, &mut quads, &mut indices)?;
        }

        Ok(BackgroundQuadBatch { quads, indices })
    }

    fn validate_config(&self) -> std::result::Result<(), BackgroundQuadError> {
        if self.config.cell_width_px == 0 || self.config.cell_height_px == 0 {
            return Err(BackgroundQuadError::ZeroDimension);
        }
        Ok(())
    }

    fn append_decoration(
        &self,
        decoration: PlannedTextDecoration,
        quads: &mut Vec<BackgroundQuad>,
        indices: &mut Vec<u32>,
    ) -> std::result::Result<(), BackgroundQuadError> {
        let cell_width = self.config.cell_width_px as f32;
        let cell_height = self.config.cell_height_px as f32;
        let x0 = f32::from(decoration.col) * cell_width;
        let x1 = x0 + (cell_width * f32::from(decoration.cols));
        let row_y0 = f32::from(decoration.row) * cell_height;
        let row_y1 = row_y0 + cell_height;
        let thickness = text_decoration_stroke_px(cell_height);
        let gap = thickness;
        let color_rgba = rgba8_to_normalized(decoration.color_rgba8);
        let geometry = DecorationGeometry {
            decoration,
            x0,
            x1,
            row_y1,
            thickness,
            color_rgba,
            cell_width,
        };
        match decoration.kind {
            TextDecorationKind::Underline | TextDecorationKind::DoubleUnderlineBottom => self
                .push_decoration_quad(
                    quads,
                    indices,
                    decoration_quad(decoration, x0, x1, row_y1 - thickness, row_y1, color_rgba),
                ),
            TextDecorationKind::DoubleUnderlineTop => self.push_decoration_quad(
                quads,
                indices,
                decoration_quad(
                    decoration,
                    x0,
                    x1,
                    row_y1 - (thickness * 2.0) - gap,
                    row_y1 - thickness - gap,
                    color_rgba,
                ),
            ),
            TextDecorationKind::CurlyUnderline => {
                self.append_curly_underline(quads, indices, geometry)
            }
            TextDecorationKind::DottedUnderline => {
                self.append_dotted_underline(quads, indices, geometry)
            }
            TextDecorationKind::DashedUnderline => {
                self.append_dashed_underline(quads, indices, geometry)
            }
            TextDecorationKind::Overline => self.push_decoration_quad(
                quads,
                indices,
                decoration_quad(decoration, x0, x1, row_y0, row_y0 + thickness, color_rgba),
            ),
            TextDecorationKind::Strikethrough => {
                let center = row_y0 + (cell_height / 2.0);
                let y0 = center - (thickness / 2.0);
                self.push_decoration_quad(
                    quads,
                    indices,
                    decoration_quad(decoration, x0, x1, y0, y0 + thickness, color_rgba),
                )
            }
        }
    }

    fn append_dotted_underline(
        &self,
        quads: &mut Vec<BackgroundQuad>,
        indices: &mut Vec<u32>,
        geometry: DecorationGeometry,
    ) -> std::result::Result<(), BackgroundQuadError> {
        let dot_size = geometry.thickness;
        let advance = dot_size * 2.0;
        let mut x = geometry.x0;
        while x < geometry.x1 {
            let dot_x1 = (x + dot_size).min(geometry.x1);
            self.push_decoration_quad(
                quads,
                indices,
                decoration_quad(
                    geometry.decoration,
                    x,
                    dot_x1,
                    geometry.row_y1 - geometry.thickness,
                    geometry.row_y1,
                    geometry.color_rgba,
                ),
            )?;
            x += advance;
        }
        Ok(())
    }

    fn append_dashed_underline(
        &self,
        quads: &mut Vec<BackgroundQuad>,
        indices: &mut Vec<u32>,
        geometry: DecorationGeometry,
    ) -> std::result::Result<(), BackgroundQuadError> {
        let dash_width = (geometry.cell_width * 0.75).max(geometry.thickness * 2.0);
        let advance = dash_width + (geometry.thickness * 2.0);
        let mut x = geometry.x0;
        while x < geometry.x1 {
            let dash_x1 = (x + dash_width).min(geometry.x1);
            self.push_decoration_quad(
                quads,
                indices,
                decoration_quad(
                    geometry.decoration,
                    x,
                    dash_x1,
                    geometry.row_y1 - geometry.thickness,
                    geometry.row_y1,
                    geometry.color_rgba,
                ),
            )?;
            x += advance;
        }
        Ok(())
    }

    fn append_curly_underline(
        &self,
        quads: &mut Vec<BackgroundQuad>,
        indices: &mut Vec<u32>,
        geometry: DecorationGeometry,
    ) -> std::result::Result<(), BackgroundQuadError> {
        let segment_width = (geometry.cell_width / 2.0).max(geometry.thickness * 2.0);
        let high_y = geometry.row_y1 - (geometry.thickness * 3.0);
        let low_y = geometry.row_y1 - geometry.thickness;
        let mut x = geometry.x0;
        let mut y0 = low_y;
        let mut y1 = high_y;
        while x < geometry.x1 {
            let next_x = (x + segment_width).min(geometry.x1);
            self.push_decoration_quad(
                quads,
                indices,
                decoration_segment_quad(
                    geometry.decoration,
                    [x, y0],
                    [next_x, y1],
                    geometry.thickness,
                    geometry.color_rgba,
                ),
            )?;
            x = next_x;
            std::mem::swap(&mut y0, &mut y1);
        }
        Ok(())
    }

    fn push_decoration_quad(
        &self,
        quads: &mut Vec<BackgroundQuad>,
        indices: &mut Vec<u32>,
        quad: BackgroundQuad,
    ) -> std::result::Result<(), BackgroundQuadError> {
        let base = checked_background_quad_base_index(quads.len())?;
        quads
            .try_reserve_exact(1)
            .map_err(|_| BackgroundQuadError::IndexCountTooLarge)?;
        indices
            .try_reserve_exact(6)
            .map_err(|_| BackgroundQuadError::IndexCountTooLarge)?;
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        quads.push(quad);
        Ok(())
    }
}

fn text_decoration_stroke_px(cell_height: f32) -> f32 {
    (cell_height / 10.0).ceil().clamp(1.0, cell_height)
}

#[derive(Debug, Clone, Copy)]
struct DecorationGeometry {
    decoration: PlannedTextDecoration,
    x0: f32,
    x1: f32,
    row_y1: f32,
    thickness: f32,
    color_rgba: [f32; 4],
    cell_width: f32,
}

fn decoration_quad(
    decoration: PlannedTextDecoration,
    x0: f32,
    x1: f32,
    y0: f32,
    y1: f32,
    color_rgba: [f32; 4],
) -> BackgroundQuad {
    BackgroundQuad {
        row: decoration.row,
        col: decoration.col,
        cols: decoration.cols,
        vertices: [
            BackgroundVertex {
                position: [x0, y0],
                color_rgba,
            },
            BackgroundVertex {
                position: [x1, y0],
                color_rgba,
            },
            BackgroundVertex {
                position: [x1, y1],
                color_rgba,
            },
            BackgroundVertex {
                position: [x0, y1],
                color_rgba,
            },
        ],
    }
}

fn decoration_segment_quad(
    decoration: PlannedTextDecoration,
    start: [f32; 2],
    end: [f32; 2],
    thickness: f32,
    color_rgba: [f32; 4],
) -> BackgroundQuad {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let length = (dx.mul_add(dx, dy * dy)).sqrt();
    let half_thickness = thickness / 2.0;
    let (normal_x, normal_y) = if length > 0.0 {
        (
            (-dy / length) * half_thickness,
            (dx / length) * half_thickness,
        )
    } else {
        (0.0, half_thickness)
    };
    BackgroundQuad {
        row: decoration.row,
        col: decoration.col,
        cols: decoration.cols,
        vertices: [
            BackgroundVertex {
                position: [start[0] + normal_x, start[1] + normal_y],
                color_rgba,
            },
            BackgroundVertex {
                position: [end[0] + normal_x, end[1] + normal_y],
                color_rgba,
            },
            BackgroundVertex {
                position: [end[0] - normal_x, end[1] - normal_y],
                color_rgba,
            },
            BackgroundVertex {
                position: [start[0] - normal_x, start[1] - normal_y],
                color_rgba,
            },
        ],
    }
}

/// Pixel layout used to build solid cursor quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorQuadConfig {
    /// Terminal cell width in pixels.
    pub cell_width_px: u32,
    /// Terminal cell height in pixels.
    pub cell_height_px: u32,
    /// Cursor color in RGBA8.
    pub color_rgba8: [u8; 4],
}

/// Deterministic CPU-side planner for terminal cursor quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorQuadPlanner {
    config: CursorQuadConfig,
}

impl CursorQuadPlanner {
    /// Create a cursor quad planner.
    pub fn new(config: CursorQuadConfig) -> Self {
        Self { config }
    }

    /// Build a solid cursor quad batch from a render plan.
    pub fn plan(
        &self,
        plan: &RenderPlan,
    ) -> std::result::Result<BackgroundQuadBatch, BackgroundQuadError> {
        self.validate_config()?;
        if !plan.cursor.visible
            || plan.cursor.row >= plan.viewport_rows
            || plan.cursor.col >= plan.viewport_cols
        {
            return Ok(BackgroundQuadBatch::default());
        }
        let quad = self.plan_cursor(plan.cursor);
        Ok(BackgroundQuadBatch {
            quads: vec![quad],
            indices: vec![0, 1, 2, 0, 2, 3],
        })
    }

    fn validate_config(&self) -> std::result::Result<(), BackgroundQuadError> {
        if self.config.cell_width_px == 0 || self.config.cell_height_px == 0 {
            return Err(BackgroundQuadError::ZeroDimension);
        }
        Ok(())
    }

    fn plan_cursor(&self, cursor: CursorSnapshot) -> BackgroundQuad {
        let cell_width = self.config.cell_width_px as f32;
        let cell_height = self.config.cell_height_px as f32;
        let cell_x0 = f32::from(cursor.col) * cell_width;
        let cell_y0 = f32::from(cursor.row) * cell_height;
        let cell_x1 = cell_x0 + cell_width;
        let cell_y1 = cell_y0 + cell_height;
        let (x0, y0, x1, y1) = match cursor.shape {
            CursorShape::Block => (cell_x0, cell_y0, cell_x1, cell_y1),
            CursorShape::Underline => {
                let thickness = cursor_stroke_px(cell_height);
                (cell_x0, cell_y1 - thickness, cell_x1, cell_y1)
            }
            CursorShape::Bar => {
                let thickness = cursor_stroke_px(cell_width);
                (cell_x0, cell_y0, cell_x0 + thickness, cell_y1)
            }
        };
        let color_rgba = rgba8_to_normalized(self.config.color_rgba8);

        BackgroundQuad {
            row: cursor.row,
            col: cursor.col,
            cols: 1,
            vertices: [
                BackgroundVertex {
                    position: [x0, y0],
                    color_rgba,
                },
                BackgroundVertex {
                    position: [x1, y0],
                    color_rgba,
                },
                BackgroundVertex {
                    position: [x1, y1],
                    color_rgba,
                },
                BackgroundVertex {
                    position: [x0, y1],
                    color_rgba,
                },
            ],
        }
    }
}

fn cursor_stroke_px(cell_dimension: f32) -> f32 {
    (cell_dimension / 8.0).ceil().clamp(1.0, cell_dimension)
}

/// Pixel and atlas layout used to build textured glyph quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphQuadConfig {
    /// Terminal cell width in pixels.
    pub cell_width_px: u32,
    /// Terminal cell height in pixels.
    pub cell_height_px: u32,
    /// Glyph atlas slot width in pixels.
    pub atlas_slot_width_px: u32,
    /// Glyph atlas slot height in pixels.
    pub atlas_slot_height_px: u32,
    /// Number of atlas slots per row.
    pub atlas_columns: u32,
    /// Atlas texture width in pixels.
    pub atlas_width_px: u32,
    /// Atlas texture height in pixels.
    pub atlas_height_px: u32,
}

/// Errors produced while building textured glyph quads.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GlyphQuadError {
    /// Pixel or atlas dimensions must be non-zero.
    #[error("glyph quad dimensions must be non-zero")]
    ZeroDimension,
    /// The planned glyph batch cannot be represented in `u32` GPU indices.
    #[error("glyph quad count is too large for u32 GPU indices")]
    IndexCountTooLarge,
    /// A glyph atlas slot falls outside the configured atlas texture.
    #[error("glyph atlas slot {slot} is outside the configured atlas image")]
    SlotOutsideAtlas {
        /// Atlas slot index that could not be represented.
        slot: u32,
    },
}

/// One vertex for a textured glyph quad.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphVertex {
    /// Pixel-space output position.
    pub position: [f32; 2],
    /// Atlas texture coordinate.
    pub uv: [f32; 2],
    /// Foreground text color in normalized RGBA.
    pub foreground_rgba: [f32; 4],
}

/// One textured glyph quad derived from a planned glyph.
#[derive(Debug, Clone, PartialEq)]
pub struct GlyphQuad {
    /// Full terminal cell text represented by this quad.
    pub text: String,
    /// Character represented by this quad.
    pub ch: char,
    /// Atlas entry sampled by this quad.
    pub atlas_entry: GlyphEntry,
    /// Quad vertices in top-left, top-right, bottom-right, bottom-left order.
    pub vertices: [GlyphVertex; 4],
}

/// Indexed glyph quad batch ready for GPU vertex/index buffer upload.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GlyphQuadBatch {
    /// Textured glyph quads.
    pub quads: Vec<GlyphQuad>,
    /// Triangle indices for all quads.
    pub indices: Vec<u32>,
}

/// Deterministic CPU-side planner for terminal glyph draw quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphQuadPlanner {
    config: GlyphQuadConfig,
}

impl GlyphQuadPlanner {
    /// Create a glyph quad planner.
    pub fn new(config: GlyphQuadConfig) -> Self {
        Self { config }
    }

    /// Build textured quads and triangle indices from a render plan.
    pub fn plan(&self, plan: &RenderPlan) -> std::result::Result<GlyphQuadBatch, GlyphQuadError> {
        self.validate_config()?;
        let mut quads = Vec::new();
        quads
            .try_reserve_exact(plan.glyphs.len())
            .map_err(|_| GlyphQuadError::IndexCountTooLarge)?;
        let mut indices = Vec::new();
        indices
            .try_reserve_exact(checked_glyph_quad_index_capacity(plan.glyphs.len())?)
            .map_err(|_| GlyphQuadError::IndexCountTooLarge)?;

        for glyph in &plan.glyphs {
            let quad = self.plan_glyph(glyph)?;
            let base = checked_glyph_quad_base_index(quads.len())?;
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            quads.push(quad);
        }

        Ok(GlyphQuadBatch { quads, indices })
    }

    fn validate_config(&self) -> std::result::Result<(), GlyphQuadError> {
        if self.config.cell_width_px == 0
            || self.config.cell_height_px == 0
            || self.config.atlas_slot_width_px == 0
            || self.config.atlas_slot_height_px == 0
            || self.config.atlas_columns == 0
            || self.config.atlas_width_px == 0
            || self.config.atlas_height_px == 0
        {
            return Err(GlyphQuadError::ZeroDimension);
        }
        Ok(())
    }

    fn plan_glyph(&self, glyph: &PlannedGlyph) -> std::result::Result<GlyphQuad, GlyphQuadError> {
        let cell_width = self.config.cell_width_px as f32;
        let cell_height = self.config.cell_height_px as f32;
        let x0 = f32::from(glyph.col) * cell_width;
        let y0 = f32::from(glyph.row) * cell_height;
        let glyph_cells = if glyph.is_wide { 2.0 } else { 1.0 };
        let x1 = x0 + (cell_width * glyph_cells);
        let y1 = y0 + cell_height;

        let slot = glyph.atlas_entry.slot;
        let slot_col = slot % self.config.atlas_columns;
        let slot_row = slot / self.config.atlas_columns;
        let atlas_x0 = slot_col
            .checked_mul(self.config.atlas_slot_width_px)
            .ok_or(GlyphQuadError::SlotOutsideAtlas { slot })?;
        let atlas_y0 = slot_row
            .checked_mul(self.config.atlas_slot_height_px)
            .ok_or(GlyphQuadError::SlotOutsideAtlas { slot })?;
        let atlas_x1 = atlas_x0
            .checked_add(self.config.atlas_slot_width_px)
            .ok_or(GlyphQuadError::SlotOutsideAtlas { slot })?;
        let atlas_y1 = atlas_y0
            .checked_add(self.config.atlas_slot_height_px)
            .ok_or(GlyphQuadError::SlotOutsideAtlas { slot })?;
        if atlas_x1 > self.config.atlas_width_px || atlas_y1 > self.config.atlas_height_px {
            return Err(GlyphQuadError::SlotOutsideAtlas { slot });
        }

        let u0 = atlas_x0 as f32 / self.config.atlas_width_px as f32;
        let v0 = atlas_y0 as f32 / self.config.atlas_height_px as f32;
        let u1 = atlas_x1 as f32 / self.config.atlas_width_px as f32;
        let v1 = atlas_y1 as f32 / self.config.atlas_height_px as f32;
        let foreground_rgba = style_foreground_rgba(glyph.style);

        Ok(GlyphQuad {
            text: glyph.text.clone(),
            ch: glyph.ch,
            atlas_entry: glyph.atlas_entry,
            vertices: [
                GlyphVertex {
                    position: [x0, y0],
                    uv: [u0, v0],
                    foreground_rgba,
                },
                GlyphVertex {
                    position: [x1, y0],
                    uv: [u1, v0],
                    foreground_rgba,
                },
                GlyphVertex {
                    position: [x1, y1],
                    uv: [u1, v1],
                    foreground_rgba,
                },
                GlyphVertex {
                    position: [x0, y1],
                    uv: [u0, v1],
                    foreground_rgba,
                },
            ],
        })
    }
}

fn checked_background_quad_base_index(
    quad_index: usize,
) -> std::result::Result<u32, BackgroundQuadError> {
    u32::try_from(quad_index)
        .ok()
        .and_then(|index| index.checked_mul(4))
        .ok_or(BackgroundQuadError::IndexCountTooLarge)
}

fn checked_background_quad_index_capacity(
    quad_count: usize,
) -> std::result::Result<usize, BackgroundQuadError> {
    quad_count
        .checked_mul(6)
        .ok_or(BackgroundQuadError::IndexCountTooLarge)
}

fn checked_glyph_quad_base_index(quad_index: usize) -> std::result::Result<u32, GlyphQuadError> {
    u32::try_from(quad_index)
        .ok()
        .and_then(|index| index.checked_mul(4))
        .ok_or(GlyphQuadError::IndexCountTooLarge)
}

fn checked_glyph_quad_index_capacity(
    quad_count: usize,
) -> std::result::Result<usize, GlyphQuadError> {
    quad_count
        .checked_mul(6)
        .ok_or(GlyphQuadError::IndexCountTooLarge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_quad_base_index_accepts_last_representable_quad() {
        let last_valid_quad = usize::try_from(u32::MAX / 4).unwrap();

        assert_eq!(
            checked_glyph_quad_base_index(last_valid_quad).unwrap(),
            u32::MAX - 3
        );
    }

    #[test]
    fn glyph_quad_base_index_rejects_overflowing_quad_count() {
        let first_invalid_quad = usize::try_from(u32::MAX / 4).unwrap() + 1;

        let error = checked_glyph_quad_base_index(first_invalid_quad).unwrap_err();

        assert_eq!(error, GlyphQuadError::IndexCountTooLarge);
    }

    #[test]
    fn glyph_quad_index_capacity_uses_checked_multiplication() {
        assert_eq!(checked_glyph_quad_index_capacity(7).unwrap(), 42);

        let error = checked_glyph_quad_index_capacity((usize::MAX / 6) + 1).unwrap_err();

        assert_eq!(error, GlyphQuadError::IndexCountTooLarge);
    }
}
