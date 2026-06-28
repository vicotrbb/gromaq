const THEME_HIGH_CONTRAST_TEXT_MIN_PIXELS: usize = 64;
const WELCOME_HIGH_CONTRAST_TEXT_MIN_PIXELS: usize = 512;
const WELCOME_AVATAR_COLOR_MIN_PIXELS: usize = 256;
const AVATAR_COLOR_DISTANCE_MAX_SQUARED: u32 = 42 * 42;
const HIGH_CONTRAST_TEXT_MIN_X100: u64 = 700;

mod color;

use color::{contrast_ratio_x100, linear_f64_rgba_to_srgb8};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ThemePreviewPixelReport {
    pub(super) high_contrast_text_pixels: usize,
    pub(super) selection_pixels: usize,
    pub(super) cursor_pixels: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WelcomePreviewPixelReport {
    pub(super) high_contrast_text_pixels: usize,
    pub(super) avatar_color_pixels: usize,
}

pub(super) fn validate_theme_preview_pixels(
    pixels: &[u8],
    width: u32,
    clear_color: [f64; 4],
    selection_color: [u8; 4],
    cursor_color: [u8; 4],
) -> Result<ThemePreviewPixelReport, String> {
    let expected = expected_background_pixel("theme preview", pixels, width, clear_color)?;
    let background_rgb = [expected[0], expected[1], expected[2]];
    let visible_selection = composited_on_background(selection_color, background_rgb);
    let visible_cursor = composited_on_background(cursor_color, background_rgb);
    let counts = count_preview_pixels(
        pixels,
        expected,
        Some(visible_selection),
        Some(visible_cursor),
    );

    if counts.high_contrast_text_pixels < THEME_HIGH_CONTRAST_TEXT_MIN_PIXELS {
        return Err(format!(
            "theme preview rendered too few high-contrast text pixels: {}",
            counts.high_contrast_text_pixels
        ));
    }
    if counts.selection_pixels == 0 {
        return Err("theme preview did not contain selection-color pixels".to_owned());
    }
    if counts.cursor_pixels == 0 {
        return Err("theme preview did not contain cursor-color pixels".to_owned());
    }
    Ok(ThemePreviewPixelReport {
        high_contrast_text_pixels: counts.high_contrast_text_pixels,
        selection_pixels: counts.selection_pixels,
        cursor_pixels: counts.cursor_pixels,
    })
}

pub(super) fn validate_welcome_preview_pixels(
    pixels: &[u8],
    width: u32,
    clear_color: [f64; 4],
    avatar_colors: &[[u8; 3]],
) -> Result<WelcomePreviewPixelReport, String> {
    let expected = expected_background_pixel("welcome preview", pixels, width, clear_color)?;
    let counts = count_preview_pixels(pixels, expected, None, None);
    let avatar_color_pixels = count_avatar_color_pixels(pixels, expected, avatar_colors);
    if counts.high_contrast_text_pixels < WELCOME_HIGH_CONTRAST_TEXT_MIN_PIXELS {
        return Err(format!(
            "welcome preview rendered too few high-contrast text pixels: {}",
            counts.high_contrast_text_pixels
        ));
    }
    if avatar_color_pixels < WELCOME_AVATAR_COLOR_MIN_PIXELS {
        return Err(format!(
            "welcome preview rendered too few avatar-color pixels: {avatar_color_pixels}"
        ));
    }
    Ok(WelcomePreviewPixelReport {
        high_contrast_text_pixels: counts.high_contrast_text_pixels,
        avatar_color_pixels,
    })
}

fn expected_background_pixel(
    label: &str,
    pixels: &[u8],
    width: u32,
    clear_color: [f64; 4],
) -> Result<[u8; 4], String> {
    let expected = linear_f64_rgba_to_srgb8(clear_color);
    let Some(pixel) = pixels.get(0..4) else {
        return Err(format!("{label} did not contain any pixels"));
    };
    if pixel != expected {
        return Err(format!(
            "{label} background pixel {:?} did not match expected {:?} at width {width}",
            pixel, expected
        ));
    }
    Ok(expected)
}

fn count_preview_pixels(
    pixels: &[u8],
    expected_background: [u8; 4],
    visible_selection: Option<[u8; 4]>,
    visible_cursor: Option<[u8; 4]>,
) -> PreviewPixelCounts {
    let mut counts = PreviewPixelCounts::default();
    let background_rgb = [
        expected_background[0],
        expected_background[1],
        expected_background[2],
    ];
    for pixel in pixels.chunks_exact(4) {
        let rgba = [pixel[0], pixel[1], pixel[2], pixel[3]];
        if Some(rgba) == visible_selection {
            counts.selection_pixels += 1;
            continue;
        }
        if Some(rgba) == visible_cursor {
            counts.cursor_pixels += 1;
            continue;
        }
        if rgba == expected_background {
            continue;
        }
        let rgb = [rgba[0], rgba[1], rgba[2]];
        if contrast_ratio_x100(rgb, background_rgb) >= HIGH_CONTRAST_TEXT_MIN_X100 {
            counts.high_contrast_text_pixels += 1;
        }
    }
    counts
}

fn count_avatar_color_pixels(
    pixels: &[u8],
    expected_background: [u8; 4],
    avatar_colors: &[[u8; 3]],
) -> usize {
    pixels
        .chunks_exact(4)
        .filter(|pixel| {
            let rgba = [pixel[0], pixel[1], pixel[2], pixel[3]];
            rgba != expected_background
                && avatar_colors.iter().any(|color| {
                    color_distance_squared([rgba[0], rgba[1], rgba[2]], *color)
                        <= AVATAR_COLOR_DISTANCE_MAX_SQUARED
                })
        })
        .count()
}

fn color_distance_squared(a: [u8; 3], b: [u8; 3]) -> u32 {
    let red = i32::from(a[0]) - i32::from(b[0]);
    let green = i32::from(a[1]) - i32::from(b[1]);
    let blue = i32::from(a[2]) - i32::from(b[2]);
    (red * red + green * green + blue * blue) as u32
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct PreviewPixelCounts {
    high_contrast_text_pixels: usize,
    selection_pixels: usize,
    cursor_pixels: usize,
}

#[cfg(test)]
mod tests;
