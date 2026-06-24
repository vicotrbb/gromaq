use crate::cell::{Color, Style};

pub(super) fn style_foreground_rgba(
    style: Style,
    default_foreground_rgb8: [u8; 3],
    ansi_colors_rgb8: [[u8; 3]; 16],
    dim_opacity: f32,
) -> [f32; 4] {
    if style.hidden {
        return [0.0, 0.0, 0.0, 0.0];
    }
    let color = if style.inverse {
        style.background
    } else {
        style.foreground
    };
    let [red, green, blue] = color_rgb8(color, default_foreground_rgb8, ansi_colors_rgb8);
    let alpha = if style.dim { dim_opacity } else { 1.0 };
    [
        srgb8_to_linear_f32(red),
        srgb8_to_linear_f32(green),
        srgb8_to_linear_f32(blue),
        alpha,
    ]
}

pub(super) fn style_background_rgba8(
    style: Style,
    default_foreground_rgb8: [u8; 3],
    ansi_colors_rgb8: [[u8; 3]; 16],
) -> Option<[u8; 4]> {
    let color = if style.inverse {
        style.foreground
    } else {
        style.background
    };
    if color == Color::Default && !style.inverse {
        return None;
    }
    let [red, green, blue] = color_rgb8(color, default_foreground_rgb8, ansi_colors_rgb8);
    Some([red, green, blue, 255])
}

pub(super) fn decoration_color_rgba8(
    decoration_color: Color,
    style: Style,
    default_foreground_rgb8: [u8; 3],
    ansi_colors_rgb8: [[u8; 3]; 16],
) -> [u8; 4] {
    let color = if decoration_color == Color::Default {
        if style.inverse {
            style.background
        } else {
            style.foreground
        }
    } else {
        decoration_color
    };
    let [red, green, blue] = color_rgb8(color, default_foreground_rgb8, ansi_colors_rgb8);
    [red, green, blue, 255]
}

pub(super) fn rgba8_to_linear_normalized([red, green, blue, alpha]: [u8; 4]) -> [f32; 4] {
    [
        srgb8_to_linear_f32(red),
        srgb8_to_linear_f32(green),
        srgb8_to_linear_f32(blue),
        f32::from(alpha) / 255.0,
    ]
}

pub(super) fn rgb8_to_linear_clear_color([red, green, blue]: [u8; 3]) -> [f64; 4] {
    [
        f64::from(srgb8_to_linear_f32(red)),
        f64::from(srgb8_to_linear_f32(green)),
        f64::from(srgb8_to_linear_f32(blue)),
        1.0,
    ]
}

fn color_rgb8(
    color: Color,
    default_foreground_rgb8: [u8; 3],
    ansi_colors_rgb8: [[u8; 3]; 16],
) -> [u8; 3] {
    match color {
        Color::Default => default_foreground_rgb8,
        Color::Ansi(index) => ansi_color_rgb8(index, ansi_colors_rgb8),
        Color::Indexed(index) => indexed_color_rgb8(index, ansi_colors_rgb8),
        Color::Rgb(red, green, blue) => [red, green, blue],
    }
}

fn ansi_color_rgb8(index: u8, ansi_colors_rgb8: [[u8; 3]; 16]) -> [u8; 3] {
    ansi_colors_rgb8[usize::from(index.min(15))]
}

fn indexed_color_rgb8(index: u8, ansi_colors_rgb8: [[u8; 3]; 16]) -> [u8; 3] {
    if index < 16 {
        return ansi_color_rgb8(index, ansi_colors_rgb8);
    }
    if index < 232 {
        let value = index - 16;
        let red = value / 36;
        let green = (value % 36) / 6;
        let blue = value % 6;
        return [
            color_cube_component(red),
            color_cube_component(green),
            color_cube_component(blue),
        ];
    }
    let gray = 8 + ((index - 232) * 10);
    [gray, gray, gray]
}

fn color_cube_component(value: u8) -> u8 {
    if value == 0 { 0 } else { 55 + (value * 40) }
}

fn srgb8_to_linear_f32(value: u8) -> f32 {
    let srgb = f32::from(value) / 255.0;
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb8_to_linear_f32_preserves_srgb_endpoints() {
        assert_eq!(srgb8_to_linear_f32(0), 0.0);
        assert_eq!(srgb8_to_linear_f32(255), 1.0);
    }

    #[test]
    fn rgb8_to_linear_clear_color_keeps_dark_theme_background_visually_dark() {
        let clear = rgb8_to_linear_clear_color([11, 15, 20]);

        assert!(clear[0] < 0.004);
        assert!(clear[1] < 0.005);
        assert!(clear[2] < 0.008);
        assert_eq!(clear[3], 1.0);
    }
}
