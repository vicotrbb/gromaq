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

    let report = validate_theme_preview_pixels(&pixels, 8, BLACK_CLEAR, SELECTION, CURSOR).unwrap();

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

fn preview_pixels(text_pixels: usize, include_selection: bool, include_cursor: bool) -> Vec<u8> {
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
