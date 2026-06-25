use super::*;

#[test]
fn last_rgba_pixel_reports_checked_slice() {
    assert_eq!(
        last_rgba_pixel(&[1, 2, 3, 4], "readback").unwrap(),
        &[1, 2, 3, 4]
    );
    assert_eq!(
        last_rgba_pixel(&[1, 2, 3, 4, 5, 6, 7, 8], "readback").unwrap(),
        &[5, 6, 7, 8]
    );
}

#[test]
fn last_rgba_pixel_rejects_short_buffers() {
    let error = last_rgba_pixel(&[1, 2, 3], "readback").unwrap_err();

    assert_eq!(
        error,
        GpuBootstrapError::SmokeReadback("readback is shorter than one RGBA pixel".to_owned())
    );
}

#[test]
fn rgba_pixel_at_reports_checked_slice() {
    assert_eq!(
        rgba_pixel_at(&[1, 2, 3, 4, 5, 6, 7, 8], 1, "pixel").unwrap(),
        &[5, 6, 7, 8]
    );
}

#[test]
fn rgba_pixel_at_rejects_missing_pixel() {
    let error = rgba_pixel_at(&[1, 2, 3, 4], 1, "pixel").unwrap_err();

    assert_eq!(
        error,
        GpuBootstrapError::SmokeReadback("pixel is missing from readback".to_owned())
    );
}

#[test]
fn rgba_pixel_at_rejects_overflowing_offset() {
    let error = rgba_pixel_at(&[], usize::MAX, "pixel").unwrap_err();

    assert_eq!(
        error,
        GpuBootstrapError::SmokeReadback("pixel byte offset is too large".to_owned())
    );
}
