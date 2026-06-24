//! Native GPU smoke runner implementations.

use super::readback::{last_rgba_pixel, rgba_pixel_at};
use super::reports::{
    GpuGlyphAtlasUploadReport, GpuGlyphAtlasUploadRunner, GpuSmokeReport, GpuSmokeRunner,
    GpuTextAtlasUploadReport, GpuTextAtlasUploadRunner, GpuTextureUploadReport,
    GpuTextureUploadRunner, GpuTexturedQuadReport, GpuTexturedQuadRunner,
};
use super::text_smoke::build_text_atlas_smoke_image;
use super::{GpuBootstrapError, NativeGpuContext, UploadPattern};
use crate::renderer::GlyphAtlasImage;

mod terminal_text;

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
