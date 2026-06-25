pub(crate) fn rgba(red: u8, green: u8, blue: u8, alpha: f32) -> [f32; 4] {
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

pub(crate) fn triangle_indices_for_quads(quad_count: usize) -> Vec<u32> {
    let mut indices = Vec::with_capacity(quad_count * 6);
    for quad_index in 0..quad_count {
        let base = u32::try_from(quad_index * 4).unwrap();
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    indices
}
