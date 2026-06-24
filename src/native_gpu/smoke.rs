//! Native GPU smoke runner implementations.

use super::readback::{last_rgba_pixel, rgba_pixel_at};
use super::reports::{
    GpuGlyphAtlasUploadReport, GpuGlyphAtlasUploadRunner, GpuSmokeReport, GpuSmokeRunner,
    GpuTerminalTextReport, GpuTerminalTextRunner, GpuTextAtlasUploadReport,
    GpuTextAtlasUploadRunner, GpuTextureUploadReport, GpuTextureUploadRunner,
    GpuTexturedQuadReport, GpuTexturedQuadRunner,
};
use super::text_smoke::{build_text_atlas_smoke_frame, build_text_atlas_smoke_image};
use super::{GlyphDrawInput, GpuBootstrapError, NativeGpuContext, UploadPattern};
use crate::renderer::{
    BackgroundQuadBatch, BackgroundQuadConfig, BackgroundQuadPlanner, CursorQuadConfig,
    CursorQuadPlanner, GlyphAtlasImage, GlyphQuadConfig, GlyphQuadPlanner,
    TextDecorationQuadConfig, TextDecorationQuadPlanner,
};

impl GpuSmokeRunner for NativeGpuContext {
    fn run_smoke(&self) -> std::result::Result<GpuSmokeReport, GpuBootstrapError> {
        let width = 4;
        let height = 4;
        let pixels = self.clear_offscreen_rgba8(width, height, [0.1, 0.2, 0.3, 1.0])?;
        let first_pixel = pixels
            .get(0..4)
            .ok_or_else(|| GpuBootstrapError::SmokeReadback("empty readback".to_owned()))?;
        Ok(GpuSmokeReport {
            width,
            height,
            first_pixel: [
                first_pixel[0],
                first_pixel[1],
                first_pixel[2],
                first_pixel[3],
            ],
            nonzero_bytes: pixels.iter().filter(|byte| **byte != 0).count(),
        })
    }
}

impl GpuTextureUploadRunner for NativeGpuContext {
    fn run_texture_upload_smoke(
        &self,
    ) -> std::result::Result<GpuTextureUploadReport, GpuBootstrapError> {
        let pattern = UploadPattern::checker_rgba8_2x2();
        let pixels = self.upload_rgba8_and_readback(&pattern)?;
        let first_pixel = pixels
            .get(0..4)
            .ok_or_else(|| GpuBootstrapError::SmokeReadback("empty upload readback".to_owned()))?;
        let last_pixel = last_rgba_pixel(&pixels, "upload readback")?;
        let matching_bytes = pixels
            .iter()
            .zip(pattern.rgba.iter())
            .filter(|(actual, expected)| actual == expected)
            .count();
        Ok(GpuTextureUploadReport {
            width: pattern.width,
            height: pattern.height,
            first_pixel: [
                first_pixel[0],
                first_pixel[1],
                first_pixel[2],
                first_pixel[3],
            ],
            last_pixel: [last_pixel[0], last_pixel[1], last_pixel[2], last_pixel[3]],
            matching_bytes,
            total_bytes: pattern.rgba.len(),
        })
    }
}

impl GpuGlyphAtlasUploadRunner for NativeGpuContext {
    fn run_glyph_atlas_upload_smoke(
        &self,
    ) -> std::result::Result<GpuGlyphAtlasUploadReport, GpuBootstrapError> {
        let image = GlyphAtlasImage::smoke_rgba8()
            .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
        let pixels = self.upload_glyph_atlas_and_readback(&image)?;
        let first_pixel = pixels.get(0..4).ok_or_else(|| {
            GpuBootstrapError::SmokeReadback("empty glyph atlas readback".to_owned())
        })?;
        let second_slot_first_pixel = rgba_pixel_at(&pixels, 2, "second glyph slot")?;
        let matching_bytes = pixels
            .iter()
            .zip(image.rgba.iter())
            .filter(|(actual, expected)| actual == expected)
            .count();
        Ok(GpuGlyphAtlasUploadReport {
            width: image.width,
            height: image.height,
            occupied_slots: image.occupied_slots,
            first_pixel: [
                first_pixel[0],
                first_pixel[1],
                first_pixel[2],
                first_pixel[3],
            ],
            second_slot_first_pixel: [
                second_slot_first_pixel[0],
                second_slot_first_pixel[1],
                second_slot_first_pixel[2],
                second_slot_first_pixel[3],
            ],
            matching_bytes,
            total_bytes: image.rgba.len(),
        })
    }
}

impl GpuTextAtlasUploadRunner for NativeGpuContext {
    fn run_text_atlas_upload_smoke(
        &self,
    ) -> std::result::Result<GpuTextAtlasUploadReport, GpuBootstrapError> {
        let (image, batch) = build_text_atlas_smoke_image()?;
        let pixels = self.upload_glyph_atlas_and_readback(&image)?;
        let matching_bytes = pixels
            .iter()
            .zip(image.rgba.iter())
            .filter(|(actual, expected)| actual == expected)
            .count();
        let covered_pixels = image
            .rgba
            .chunks_exact(4)
            .filter(|pixel| pixel[3] != 0)
            .count();
        Ok(GpuTextAtlasUploadReport {
            width: image.width,
            height: image.height,
            occupied_slots: image.occupied_slots,
            rasterized_glyphs: batch.rasterized,
            reused_glyphs: batch.reused,
            matching_bytes,
            total_bytes: image.rgba.len(),
            covered_pixels,
        })
    }
}

impl GpuTexturedQuadRunner for NativeGpuContext {
    fn run_textured_quad_smoke(
        &self,
    ) -> std::result::Result<GpuTexturedQuadReport, GpuBootstrapError> {
        let width = 4;
        let height = 4;
        let pixels = self.draw_textured_quad_and_readback(
            &UploadPattern::checker_rgba8_2x2(),
            width,
            height,
        )?;
        let first_pixel = pixels.get(0..4).ok_or_else(|| {
            GpuBootstrapError::SmokeReadback("empty textured quad readback".to_owned())
        })?;
        Ok(GpuTexturedQuadReport {
            width,
            height,
            first_pixel: [
                first_pixel[0],
                first_pixel[1],
                first_pixel[2],
                first_pixel[3],
            ],
            drawn_pixels: pixels.chunks_exact(4).filter(|pixel| pixel[3] != 0).count(),
        })
    }
}

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
        let pixels = super::draw_glyph_quads_rgba8(
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
