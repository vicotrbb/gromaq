const HIGH_CONTRAST_TEXT_MIN_PIXELS: usize = 64;
const HIGH_CONTRAST_TEXT_MIN_X100: u64 = 700;

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
    for pixel in pixels.chunks_exact(4) {
        let rgba = [pixel[0], pixel[1], pixel[2], pixel[3]];
        if rgba == selection_color {
            selection_pixels += 1;
            continue;
        }
        if rgba == cursor_color {
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

fn linear_f64_rgba_to_srgb8([red, green, blue, alpha]: [f64; 4]) -> [u8; 4] {
    [
        linear_channel_to_srgb8(red),
        linear_channel_to_srgb8(green),
        linear_channel_to_srgb8(blue),
        linear_channel_to_srgb8(alpha),
    ]
}

fn linear_channel_to_srgb8(value: f64) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let srgb = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        (1.055 * value.powf(1.0 / 2.4)) - 0.055
    };
    (srgb * 255.0).round().clamp(0.0, 255.0) as u8
}

fn contrast_ratio_x100(foreground: [u8; 3], background: [u8; 3]) -> u64 {
    let foreground = relative_luminance(foreground);
    let background = relative_luminance(background);
    let lighter = foreground.max(background);
    let darker = foreground.min(background);
    (((lighter + 0.05) / (darker + 0.05)) * 100.0).round() as u64
}

fn relative_luminance([red, green, blue]: [u8; 3]) -> f64 {
    let [red, green, blue] = [
        srgb_component(red),
        srgb_component(green),
        srgb_component(blue),
    ];
    (0.2126 * red) + (0.7152 * green) + (0.0722 * blue)
}

fn srgb_component(value: u8) -> f64 {
    let value = f64::from(value) / 255.0;
    if value <= 0.039_28 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLACK_CLEAR: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
    const BACKGROUND: [u8; 4] = [0, 0, 0, 255];
    const WHITE_TEXT: [u8; 4] = [255, 255, 255, 255];
    const SELECTION: [u8; 4] = [47, 59, 82, 255];
    const CURSOR: [u8; 4] = [246, 193, 119, 255];

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
        for _ in 0..text_pixels {
            pixels.extend_from_slice(&WHITE_TEXT);
        }
        if include_selection {
            pixels.extend_from_slice(&SELECTION);
        }
        if include_cursor {
            pixels.extend_from_slice(&CURSOR);
        }
        pixels
    }
}
