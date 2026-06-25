use std::path::Path;

mod metrics;
mod perf;
mod snapshot;
#[cfg(test)]
mod tests;

use super::super::reports::{
    GpuTerminalTextReport, GpuTerminalTextRunner, GpuTerminalTextSnapshotReport,
    GpuTerminalTextSnapshotRunner,
};
use super::super::text_smoke::build_text_atlas_smoke_frame;
use super::super::{GlyphDrawInput, GpuBootstrapError, NativeGpuContext, draw_glyph_quads_rgba8};
use crate::renderer::{
    BackgroundQuadBatch, BackgroundQuadConfig, BackgroundQuadPlanner, CursorQuadConfig,
    CursorQuadPlanner, GlyphQuadConfig, GlyphQuadPlanner, TextDecorationQuadConfig,
    TextDecorationQuadPlanner,
};
use metrics::terminal_text_report_from_pixels;
#[cfg(test)]
use perf::{average_duration_ns, p95_duration_ns};
use snapshot::terminal_text_ppm_bytes;

const MIN_TERMINAL_TEXT_CONTRAST_X100: u32 = 700;

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
        terminal_text_report_from_pixels(&draw, &pixels)
    }
}

impl GpuTerminalTextSnapshotRunner for NativeGpuContext {
    fn run_terminal_text_snapshot(
        &self,
        path: &Path,
    ) -> std::result::Result<GpuTerminalTextSnapshotReport, GpuBootstrapError> {
        let draw = build_terminal_text_smoke_draw()?;
        let pixels = self.draw_terminal_text_smoke_frame(&draw)?;
        let report = terminal_text_report_from_pixels(&draw, &pixels)?;
        let snapshot = terminal_text_ppm_bytes(report.width, report.height, &pixels)?;
        std::fs::write(path, &snapshot).map_err(|error| {
            GpuBootstrapError::SmokeReadback(format!(
                "failed to write terminal text snapshot to {}: {error}",
                path.display()
            ))
        })?;
        Ok(GpuTerminalTextSnapshotReport {
            width: report.width,
            height: report.height,
            bytes_written: snapshot.len(),
            glyphs: report.glyphs,
            background_pixel: report.background_pixel,
            glyph_pixel: report.glyph_pixel,
            glyph_background_contrast_x100: report.glyph_background_contrast_x100,
            cursor_pixel: report.cursor_pixel,
            drawn_pixels: report.drawn_pixels,
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
