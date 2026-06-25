pub(super) fn preview_pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let index = ((y * width + x) * 4) as usize;
    [
        rgba[index],
        rgba[index + 1],
        rgba[index + 2],
        rgba[index + 3],
    ]
}

pub(super) fn rgb([red, green, blue]: [u8; 3]) -> [u8; 4] {
    [red, green, blue, 255]
}

pub(super) fn rgba(red: u8, green: u8, blue: u8, alpha: f32) -> [f32; 4] {
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
