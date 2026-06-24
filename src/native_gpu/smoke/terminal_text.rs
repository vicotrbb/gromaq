use super::super::readback::rgba_pixel_at;
use super::super::reports::{GpuTerminalTextReport, GpuTerminalTextRunner};
use super::super::text_smoke::build_text_atlas_smoke_frame;
use super::super::{GlyphDrawInput, GpuBootstrapError, NativeGpuContext, draw_glyph_quads_rgba8};
use crate::renderer::{
    BackgroundQuadBatch, BackgroundQuadConfig, BackgroundQuadPlanner, CursorQuadConfig,
    CursorQuadPlanner, GlyphQuadConfig, GlyphQuadPlanner, TextDecorationQuadConfig,
    TextDecorationQuadPlanner,
};

impl GpuTerminalTextRunner for NativeGpuContext {
    fn run_terminal_text_smoke(
        &self,
    ) -> std::result::Result<GpuTerminalTextReport, GpuBootstrapError> {
        let frame = build_text_atlas_smoke_frame()?;
        let quad_config = GlyphQuadConfig {
            cell_width_px: frame.slot_width,
            cell_height_px: frame.slot_height,
            atlas_slot_width_px: frame.slot_width,
            atlas_slot_height_px: frame.slot_height,
            atlas_columns: frame.atlas_columns,
            atlas_width_px: frame.image.width,
            atlas_height_px: frame.image.height,
        };
        let quad_batch = GlyphQuadPlanner::new(quad_config)
            .plan(&frame.plan)
            .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
        let background_batch = BackgroundQuadPlanner::new(BackgroundQuadConfig {
            cell_width_px: frame.slot_width,
            cell_height_px: frame.slot_height,
        })
        .plan(&frame.plan)
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
        let decoration_batch = TextDecorationQuadPlanner::new(TextDecorationQuadConfig {
            cell_width_px: frame.slot_width,
            cell_height_px: frame.slot_height,
        })
        .plan(&frame.plan)
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
        let cursor_batch = CursorQuadPlanner::new(CursorQuadConfig {
            cell_width_px: frame.slot_width,
            cell_height_px: frame.slot_height,
            color_rgba8: [229, 229, 229, 255],
        })
        .plan(&frame.plan)
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
        let (target_width, target_height) = checked_terminal_text_target_dimensions(
            frame.plan.viewport_cols,
            frame.plan.viewport_rows,
            frame.slot_width,
            frame.slot_height,
        )?;
        let pixels = draw_glyph_quads_rgba8(
            self.device(),
            self.queue(),
            GlyphDrawInput {
                image: &frame.image,
                background_batch: &background_batch,
                batch: &quad_batch,
                decoration_batch: &decoration_batch,
                cursor_batch: &cursor_batch,
                width: target_width,
                height: target_height,
            },
        )?;
        Ok(GpuTerminalTextReport {
            width: target_width,
            height: target_height,
            glyphs: frame.plan.glyphs.len(),
            background_quads: background_batch.quads.len(),
            quads: quad_batch.quads.len(),
            decoration_quads: decoration_batch.quads.len(),
            cursor_quads: cursor_batch.quads.len(),
            rasterized_glyphs: frame.batch.rasterized,
            reused_glyphs: frame.batch.reused,
            first_drawn_pixel: first_nontransparent_pixel(&pixels),
            cursor_pixel: first_cursor_pixel(&cursor_batch, &pixels, target_width)?,
            drawn_pixels: pixels.chunks_exact(4).filter(|pixel| pixel[3] != 0).count(),
        })
    }
}

fn checked_terminal_text_target_dimensions(
    cols: u16,
    rows: u16,
    slot_width: u32,
    slot_height: u32,
) -> std::result::Result<(u32, u32), GpuBootstrapError> {
    let width = u32::from(cols).checked_mul(slot_width).ok_or_else(|| {
        GpuBootstrapError::SmokeReadback(
            "terminal text target width is too large to represent".to_owned(),
        )
    })?;
    let height = u32::from(rows).checked_mul(slot_height).ok_or_else(|| {
        GpuBootstrapError::SmokeReadback(
            "terminal text target height is too large to represent".to_owned(),
        )
    })?;
    Ok((width, height))
}

fn first_nontransparent_pixel(pixels: &[u8]) -> [u8; 4] {
    pixels
        .chunks_exact(4)
        .find(|pixel| pixel[3] != 0)
        .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
        .unwrap_or([0, 0, 0, 0])
}

fn first_cursor_pixel(
    cursor_batch: &BackgroundQuadBatch,
    pixels: &[u8],
    width: u32,
) -> std::result::Result<[u8; 4], GpuBootstrapError> {
    let Some(cursor) = cursor_batch.quads.first() else {
        return Ok([0, 0, 0, 0]);
    };
    let x = cursor.vertices[0].position[0] as u32;
    let y = cursor.vertices[0].position[1] as u32;
    let pixel_index =
        usize::try_from(u64::from(y) * u64::from(width) + u64::from(x)).map_err(|_| {
            GpuBootstrapError::SmokeReadback("cursor pixel offset is too large".to_owned())
        })?;
    let pixel = rgba_pixel_at(pixels, pixel_index, "cursor pixel")?;
    Ok([pixel[0], pixel[1], pixel[2], pixel[3]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_text_target_dimensions_reports_checked_size() {
        let dimensions = checked_terminal_text_target_dimensions(80, 24, 8, 16).unwrap();

        assert_eq!(dimensions, (640, 384));
    }

    #[test]
    fn terminal_text_target_dimensions_rejects_overflowing_width() {
        let error = checked_terminal_text_target_dimensions(2, 1, u32::MAX, 1).unwrap_err();

        assert_eq!(
            error,
            GpuBootstrapError::SmokeReadback(
                "terminal text target width is too large to represent".to_owned()
            )
        );
    }

    #[test]
    fn terminal_text_target_dimensions_rejects_overflowing_height() {
        let error = checked_terminal_text_target_dimensions(1, 2, 1, u32::MAX).unwrap_err();

        assert_eq!(
            error,
            GpuBootstrapError::SmokeReadback(
                "terminal text target height is too large to represent".to_owned()
            )
        );
    }
}
