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
mod tests {
    use super::*;

    const BLACK_CLEAR: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
    const TRANSLUCENT_BLACK_CLEAR: [f64; 4] = [0.0, 0.0, 0.0, 0.75];
    const BACKGROUND: [u8; 4] = [0, 0, 0, 255];
    const TRANSLUCENT_BACKGROUND: [u8; 4] = [0, 0, 0, 191];
    const WHITE_TEXT: [u8; 4] = [255, 255, 255, 255];
    const SELECTION: [u8; 4] = [47, 59, 82, 255];
    const CURSOR: [u8; 4] = [246, 193, 119, 255];
    const TRANSLUCENT_SELECTION: [u8; 4] = [47, 59, 82, 64];
    const TRANSLUCENT_CURSOR: [u8; 4] = [246, 193, 119, 128];

    #[test]
    fn preview_pixel_validation_counts_text_selection_and_cursor_pixels() {
        let pixels = preview_pixels(64, true, true);

        let report =
            validate_theme_preview_pixels(&pixels, 8, BLACK_CLEAR, SELECTION, CURSOR).unwrap();

        assert_eq!(report.high_contrast_text_pixels, 64);
        assert_eq!(report.selection_pixels, 1);
        assert_eq!(report.cursor_pixels, 1);
    }

    #[test]
    fn preview_pixel_validation_uses_straight_alpha_for_background_opacity() {
        let pixels = preview_pixels_with_background(TRANSLUCENT_BACKGROUND, 64, true, true);

        let report =
            validate_theme_preview_pixels(&pixels, 8, TRANSLUCENT_BLACK_CLEAR, SELECTION, CURSOR)
                .unwrap();

        assert_eq!(report.high_contrast_text_pixels, 64);
    }

    #[test]
    fn preview_pixel_validation_counts_translucent_selection_and_cursor_pixels() {
        let mut pixels = Vec::new();
        pixels.extend_from_slice(&BACKGROUND);
        for _ in 0..64 {
            pixels.extend_from_slice(&WHITE_TEXT);
        }
        pixels.extend_from_slice(&composited_on_background(TRANSLUCENT_SELECTION, [0, 0, 0]));
        pixels.extend_from_slice(&composited_on_background(TRANSLUCENT_CURSOR, [0, 0, 0]));

        let report = validate_theme_preview_pixels(
            &pixels,
            8,
            BLACK_CLEAR,
            TRANSLUCENT_SELECTION,
            TRANSLUCENT_CURSOR,
        )
        .unwrap();

        assert_eq!(report.selection_pixels, 1);
        assert_eq!(report.cursor_pixels, 1);
    }

    #[test]
    fn preview_pixel_validation_rejects_low_contrast_text_coverage() {
        let pixels = preview_pixels(63, true, true);

        let error =
            validate_theme_preview_pixels(&pixels, 8, BLACK_CLEAR, SELECTION, CURSOR).unwrap_err();

        assert_eq!(
            error,
            "theme preview rendered too few high-contrast text pixels: 63"
        );
    }

    #[test]
    fn preview_pixel_validation_rejects_missing_cursor_pixels() {
        let pixels = preview_pixels(64, true, false);

        let error =
            validate_theme_preview_pixels(&pixels, 8, BLACK_CLEAR, SELECTION, CURSOR).unwrap_err();

        assert_eq!(error, "theme preview did not contain cursor-color pixels");
    }

    #[test]
    fn preview_pixel_validation_rejects_wrong_background_pixel() {
        let mut pixels = preview_pixels(64, true, true);
        pixels[..4].copy_from_slice(&WHITE_TEXT);

        let error =
            validate_theme_preview_pixels(&pixels, 8, BLACK_CLEAR, SELECTION, CURSOR).unwrap_err();

        assert!(error.contains("did not match expected"));
    }

    fn preview_pixels(
        text_pixels: usize,
        include_selection: bool,
        include_cursor: bool,
    ) -> Vec<u8> {
        let mut pixels = Vec::new();
        pixels.extend_from_slice(&BACKGROUND);
        append_preview_pixels(&mut pixels, text_pixels, include_selection, include_cursor);
        pixels
    }

    fn preview_pixels_with_background(
        background: [u8; 4],
        text_pixels: usize,
        include_selection: bool,
        include_cursor: bool,
    ) -> Vec<u8> {
        let mut pixels = Vec::new();
        pixels.extend_from_slice(&background);
        append_preview_pixels(&mut pixels, text_pixels, include_selection, include_cursor);
        pixels
    }

    fn append_preview_pixels(
        pixels: &mut Vec<u8>,
        text_pixels: usize,
        include_selection: bool,
        include_cursor: bool,
    ) {
        for _ in 0..text_pixels {
            pixels.extend_from_slice(&WHITE_TEXT);
        }
        if include_selection {
            pixels.extend_from_slice(&SELECTION);
        }
        if include_cursor {
            pixels.extend_from_slice(&CURSOR);
        }
    }
}
