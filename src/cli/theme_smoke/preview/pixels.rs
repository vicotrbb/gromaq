const HIGH_CONTRAST_TEXT_MIN_PIXELS: usize = 64;
const HIGH_CONTRAST_TEXT_MIN_X100: u64 = 700;

mod color;

use color::{contrast_ratio_x100, linear_f64_rgba_to_srgb8};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ThemePreviewPixelReport {
    pub(super) high_contrast_text_pixels: usize,
    pub(super) selection_pixels: usize,
    pub(super) cursor_pixels: usize,
}

pub(super) fn validate_theme_preview_pixels(
    pixels: &[u8],
    width: u32,
    clear_color: [f64; 4],
    selection_color: [u8; 4],
    cursor_color: [u8; 4],
) -> Result<ThemePreviewPixelReport, String> {
    let expected = linear_f64_rgba_to_srgb8(clear_color);
    let Some(pixel) = pixels.get(0..4) else {
        return Err("theme preview did not contain any pixels".to_owned());
    };
    if pixel != expected {
        return Err(format!(
            "theme preview background pixel {:?} did not match expected {:?} at width {width}",
            pixel, expected
        ));
    }

    let mut high_contrast_text_pixels = 0;
    let mut selection_pixels = 0;
    let mut cursor_pixels = 0;
    let background_rgb = [expected[0], expected[1], expected[2]];
    let visible_selection = composited_on_background(selection_color, background_rgb);
    let visible_cursor = composited_on_background(cursor_color, background_rgb);
    for pixel in pixels.chunks_exact(4) {
        let rgba = [pixel[0], pixel[1], pixel[2], pixel[3]];
        if rgba == visible_selection {
            selection_pixels += 1;
            continue;
        }
        if rgba == visible_cursor {
            cursor_pixels += 1;
            continue;
        }
        if rgba == expected {
            continue;
        }
        let rgb = [rgba[0], rgba[1], rgba[2]];
        if contrast_ratio_x100(rgb, background_rgb) >= HIGH_CONTRAST_TEXT_MIN_X100 {
            high_contrast_text_pixels += 1;
        }
    }

    if high_contrast_text_pixels < HIGH_CONTRAST_TEXT_MIN_PIXELS {
        return Err(format!(
            "theme preview rendered too few high-contrast text pixels: {high_contrast_text_pixels}"
        ));
    }
    if selection_pixels == 0 {
        return Err("theme preview did not contain selection-color pixels".to_owned());
    }
    if cursor_pixels == 0 {
        return Err("theme preview did not contain cursor-color pixels".to_owned());
    }
    Ok(ThemePreviewPixelReport {
        high_contrast_text_pixels,
        selection_pixels,
        cursor_pixels,
    })
}

fn composited_on_background([red, green, blue, alpha]: [u8; 4], background: [u8; 3]) -> [u8; 4] {
    let alpha = f32::from(alpha) / 255.0;
    let inverse = 1.0 - alpha;
    [
        blend_channel(red, background[0], alpha, inverse),
        blend_channel(green, background[1], alpha, inverse),
        blend_channel(blue, background[2], alpha, inverse),
        255,
    ]
}

fn blend_channel(src: u8, dst: u8, alpha: f32, inverse: f32) -> u8 {
    ((f32::from(src) * alpha) + (f32::from(dst) * inverse))
        .round()
        .clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests;
