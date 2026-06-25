pub(super) fn linear_f64_rgba_to_srgb8(color: [f64; 4]) -> [u8; 4] {
    [
        linear_to_srgb8(color[0] as f32),
        linear_to_srgb8(color[1] as f32),
        linear_to_srgb8(color[2] as f32),
        normalized_to_u8(color[3] as f32),
    ]
}

pub(super) fn linear_f32_rgba_to_srgb8(color: [f32; 4]) -> [u8; 4] {
    [
        linear_to_srgb8(color[0]),
        linear_to_srgb8(color[1]),
        linear_to_srgb8(color[2]),
        normalized_to_u8(color[3]),
    ]
}

pub(super) fn blend_channel(src: u8, dst: u8, alpha: f32, inverse: f32) -> u8 {
    ((f32::from(src) * alpha) + (f32::from(dst) * inverse))
        .round()
        .clamp(0.0, 255.0) as u8
}

pub(super) fn is_grayscale(pixel: [u8; 4]) -> bool {
    let red_green = pixel[0].abs_diff(pixel[1]);
    let green_blue = pixel[1].abs_diff(pixel[2]);
    red_green.saturating_add(green_blue) <= 8
}

pub(super) fn multiply_u8(lhs: u8, rhs: u8) -> u8 {
    ((u16::from(lhs) * u16::from(rhs) + 127) / 255) as u8
}

fn linear_to_srgb8(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let srgb = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        (1.055 * value.powf(1.0 / 2.4)) - 0.055
    };
    normalized_to_u8(srgb)
}

fn normalized_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}
