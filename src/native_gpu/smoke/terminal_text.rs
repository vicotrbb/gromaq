use std::time::Instant;

use super::super::readback::rgba_pixel_at;
use super::super::reports::{
    GpuTerminalTextPerfReport, GpuTerminalTextPerfRunner, GpuTerminalTextReport,
    GpuTerminalTextRunner,
};
use super::super::text_smoke::build_text_atlas_smoke_frame;
use super::super::{GlyphDrawInput, GpuBootstrapError, NativeGpuContext, draw_glyph_quads_rgba8};
use crate::renderer::{
    BackgroundQuadBatch, BackgroundQuadConfig, BackgroundQuadPlanner, CursorQuadConfig,
    CursorQuadPlanner, GlyphQuadConfig, GlyphQuadPlanner, TextDecorationQuadConfig,
    TextDecorationQuadPlanner,
};

const TERMINAL_TEXT_PERF_SMOKE_FRAMES: usize = 16;

struct TerminalTextSmokeDraw {
    frame: super::super::text_smoke::TextAtlasSmokeFrame,
    quad_batch: crate::renderer::GlyphQuadBatch,
    background_batch: BackgroundQuadBatch,
    decoration_batch: BackgroundQuadBatch,
    cursor_batch: BackgroundQuadBatch,
    target_width: u32,
    target_height: u32,
}

impl GpuTerminalTextRunner for NativeGpuContext {
    fn run_terminal_text_smoke(
        &self,
    ) -> std::result::Result<GpuTerminalTextReport, GpuBootstrapError> {
        let draw = build_terminal_text_smoke_draw()?;
        let pixels = self.draw_terminal_text_smoke_frame(&draw)?;
        Ok(GpuTerminalTextReport {
            width: draw.target_width,
            height: draw.target_height,
            glyphs: draw.frame.plan.glyphs.len(),
            background_quads: draw.background_batch.quads.len(),
            quads: draw.quad_batch.quads.len(),
            decoration_quads: draw.decoration_batch.quads.len(),
            cursor_quads: draw.cursor_batch.quads.len(),
            rasterized_glyphs: draw.frame.batch.rasterized,
            reused_glyphs: draw.frame.batch.reused,
            first_drawn_pixel: first_nontransparent_pixel(&pixels),
            cursor_pixel: first_cursor_pixel(&draw.cursor_batch, &pixels, draw.target_width)?,
            drawn_pixels: pixels.chunks_exact(4).filter(|pixel| pixel[3] != 0).count(),
        })
    }
}

impl GpuTerminalTextPerfRunner for NativeGpuContext {
    fn run_terminal_text_perf_smoke(
        &self,
    ) -> std::result::Result<GpuTerminalTextPerfReport, GpuBootstrapError> {
        let draw = build_terminal_text_smoke_draw()?;
        let mut durations = Vec::new();
        durations
            .try_reserve_exact(TERMINAL_TEXT_PERF_SMOKE_FRAMES)
            .map_err(|_| {
                GpuBootstrapError::SmokeReadback("perf sample allocation failed".to_owned())
            })?;
        let mut final_drawn_pixels = 0;

        for _ in 0..TERMINAL_TEXT_PERF_SMOKE_FRAMES {
            let started = Instant::now();
            let pixels = self.draw_terminal_text_smoke_frame(&draw)?;
            durations.push(started.elapsed().as_nanos());
            final_drawn_pixels = pixels.chunks_exact(4).filter(|pixel| pixel[3] != 0).count();
        }

        Ok(GpuTerminalTextPerfReport {
            frames: TERMINAL_TEXT_PERF_SMOKE_FRAMES,
            width: draw.target_width,
            height: draw.target_height,
            drawn_pixels: final_drawn_pixels,
            min_ns: *durations.iter().min().unwrap_or(&0),
            avg_ns: average_duration_ns(&durations),
            max_ns: *durations.iter().max().unwrap_or(&0),
            p95_ns: p95_duration_ns(&durations),
        })
    }
}

impl NativeGpuContext {
    fn draw_terminal_text_smoke_frame(
        &self,
        draw: &TerminalTextSmokeDraw,
    ) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
        draw_glyph_quads_rgba8(
            self.device(),
            self.queue(),
            GlyphDrawInput {
                image: &draw.frame.image,
                background_batch: &draw.background_batch,
                batch: &draw.quad_batch,
                decoration_batch: &draw.decoration_batch,
                cursor_batch: &draw.cursor_batch,
                width: draw.target_width,
                height: draw.target_height,
            },
        )
    }
}

fn build_terminal_text_smoke_draw() -> std::result::Result<TerminalTextSmokeDraw, GpuBootstrapError>
{
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
    Ok(TerminalTextSmokeDraw {
        frame,
        quad_batch,
        background_batch,
        decoration_batch,
        cursor_batch,
        target_width,
        target_height,
    })
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

fn average_duration_ns(durations: &[u128]) -> u128 {
    if durations.is_empty() {
        return 0;
    }
    durations.iter().sum::<u128>() / durations.len() as u128
}

fn p95_duration_ns(durations: &[u128]) -> u128 {
    if durations.is_empty() {
        return 0;
    }
    let mut sorted = durations.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() * 95).div_ceil(100)).saturating_sub(1);
    sorted[index]
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

    #[test]
    fn terminal_text_perf_average_reports_zero_without_samples() {
        assert_eq!(average_duration_ns(&[]), 0);
    }

    #[test]
    fn terminal_text_perf_p95_reports_zero_without_samples() {
        assert_eq!(p95_duration_ns(&[]), 0);
    }

    #[test]
    fn terminal_text_perf_p95_uses_inclusive_rank() {
        assert_eq!(p95_duration_ns(&[10, 20, 30, 40, 50]), 50);
    }
}
