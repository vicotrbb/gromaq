pub(super) fn linear_f64_rgba_to_srgb8([red, green, blue, alpha]: [f64; 4]) -> [u8; 4] {
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

pub(super) fn contrast_ratio_x100(foreground: [u8; 3], background: [u8; 3]) -> u64 {
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
