use super::super::super::GpuBootstrapError;
use super::super::super::readback::rgba_pixel_at;
use super::super::super::reports::GpuTerminalTextReport;
use super::{MIN_TERMINAL_TEXT_CONTRAST_X100, TerminalTextSmokeDraw};
use crate::renderer::BackgroundQuadBatch;

pub(super) fn terminal_text_report_from_pixels(
    draw: &TerminalTextSmokeDraw,
    pixels: &[u8],
) -> std::result::Result<GpuTerminalTextReport, GpuBootstrapError> {
    let background_pixel = first_nontransparent_pixel(pixels);
    let cursor_pixel = first_cursor_pixel(&draw.cursor_batch, pixels, draw.target_width)?;
    let glyph_pixel = first_glyph_pixel(pixels, background_pixel, cursor_pixel);
    let glyph_background_contrast_x100 = contrast_ratio_x100(glyph_pixel, background_pixel);
    if glyph_background_contrast_x100 < MIN_TERMINAL_TEXT_CONTRAST_X100 {
        return Err(GpuBootstrapError::SmokeReadback(format!(
            "terminal text contrast {glyph_background_contrast_x100} is below required {MIN_TERMINAL_TEXT_CONTRAST_X100}"
        )));
    }
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
        first_drawn_pixel: background_pixel,
        background_pixel,
        glyph_pixel,
        glyph_background_contrast_x100,
        cursor_pixel,
        drawn_pixels: pixels.chunks_exact(4).filter(|pixel| pixel[3] != 0).count(),
    })
}

fn first_nontransparent_pixel(pixels: &[u8]) -> [u8; 4] {
    pixels
        .chunks_exact(4)
        .find(|pixel| pixel[3] != 0)
        .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
        .unwrap_or([0, 0, 0, 0])
}

fn first_glyph_pixel(pixels: &[u8], background_pixel: [u8; 4], cursor_pixel: [u8; 4]) -> [u8; 4] {
    pixels
        .chunks_exact(4)
        .filter(|pixel| {
            pixel[3] >= 128
                && [pixel[0], pixel[1], pixel[2], pixel[3]] != background_pixel
                && [pixel[0], pixel[1], pixel[2], pixel[3]] != cursor_pixel
        })
        .max_by_key(|pixel| {
            contrast_ratio_x100([pixel[0], pixel[1], pixel[2], pixel[3]], background_pixel)
        })
        .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
        .unwrap_or([0, 0, 0, 0])
}

fn contrast_ratio_x100(foreground: [u8; 4], background: [u8; 4]) -> u32 {
    let foreground = relative_luminance(foreground);
    let background = relative_luminance(background);
    let lighter = foreground.max(background);
    let darker = foreground.min(background);
    (((lighter + 0.05) / (darker + 0.05)) * 100.0).round() as u32
}

fn relative_luminance([red, green, blue, _alpha]: [u8; 4]) -> f64 {
    0.2126 * linear_channel(red) + 0.7152 * linear_channel(green) + 0.0722 * linear_channel(blue)
}

fn linear_channel(value: u8) -> f64 {
    let value = f64::from(value) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
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
    fn terminal_text_contrast_ratio_reports_wcag_scaled_ratio() {
        assert_eq!(
            contrast_ratio_x100([255, 255, 255, 255], [0, 0, 0, 255]),
            2100
        );
        assert!(contrast_ratio_x100([244, 247, 251, 255], [9, 13, 18, 255]) > 1500);
    }

    #[test]
    fn terminal_text_contrast_gate_rejects_low_contrast_samples() {
        assert!(
            contrast_ratio_x100([80, 80, 80, 255], [70, 70, 70, 255])
                < MIN_TERMINAL_TEXT_CONTRAST_X100
        );
    }
}
