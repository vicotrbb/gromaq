use std::path::PathBuf;

pub(crate) fn system_mono_font() -> PathBuf {
    [
        "/System/Library/Fonts/SFNSMono.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
        "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|path| path.exists())
    .expect("expected a local system monospace font for renderer glyph frame proof")
}

pub(crate) fn linear_clear_color(red: u8, green: u8, blue: u8) -> [f64; 4] {
    [
        f64::from(srgb8_to_linear_f32(red)),
        f64::from(srgb8_to_linear_f32(green)),
        f64::from(srgb8_to_linear_f32(blue)),
        1.0,
    ]
}

pub(crate) fn linear_rgba(red: u8, green: u8, blue: u8, alpha: f32) -> [f32; 4] {
    [
        srgb8_to_linear_f32(red),
        srgb8_to_linear_f32(green),
        srgb8_to_linear_f32(blue),
        alpha,
    ]
}

fn srgb8_to_linear_f32(value: u8) -> f32 {
    let srgb = f32::from(value) / 255.0;
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}
