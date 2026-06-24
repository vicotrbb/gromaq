use crate::cell::{Color, Style};

pub(super) fn style_foreground_rgba(style: Style) -> [f32; 4] {
    if style.hidden {
        return [0.0, 0.0, 0.0, 0.0];
    }
    let color = if style.inverse {
        style.background
    } else {
        style.foreground
    };
    let [red, green, blue] = color_rgb8(color);
    let alpha = if style.dim { 0.66 } else { 1.0 };
    [
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        alpha,
    ]
}

pub(super) fn style_background_rgba8(style: Style) -> Option<[u8; 4]> {
    let color = if style.inverse {
        style.foreground
    } else {
        style.background
    };
    if color == Color::Default && !style.inverse {
        return None;
    }
    let [red, green, blue] = color_rgb8(color);
    Some([red, green, blue, 255])
}

pub(super) fn decoration_color_rgba8(decoration_color: Color, style: Style) -> [u8; 4] {
    let color = if decoration_color == Color::Default {
        if style.inverse {
            style.background
        } else {
            style.foreground
        }
    } else {
        decoration_color
    };
    let [red, green, blue] = color_rgb8(color);
    [red, green, blue, 255]
}

pub(super) fn rgba8_to_normalized([red, green, blue, alpha]: [u8; 4]) -> [f32; 4] {
    [
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        f32::from(alpha) / 255.0,
    ]
}

fn color_rgb8(color: Color) -> [u8; 3] {
    match color {
        Color::Default => [229, 229, 229],
        Color::Ansi(index) => ansi_color_rgb8(index),
        Color::Indexed(index) => indexed_color_rgb8(index),
        Color::Rgb(red, green, blue) => [red, green, blue],
    }
}

fn ansi_color_rgb8(index: u8) -> [u8; 3] {
    const ANSI: [[u8; 3]; 16] = [
        [0, 0, 0],
        [205, 49, 49],
        [13, 188, 121],
        [229, 229, 16],
        [36, 114, 200],
        [188, 63, 188],
        [17, 168, 205],
        [229, 229, 229],
        [102, 102, 102],
        [241, 76, 76],
        [35, 209, 139],
        [245, 245, 67],
        [59, 142, 234],
        [214, 112, 214],
        [41, 184, 219],
        [255, 255, 255],
    ];
    ANSI[usize::from(index.min(15))]
}

fn indexed_color_rgb8(index: u8) -> [u8; 3] {
    if index < 16 {
        return ansi_color_rgb8(index);
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
