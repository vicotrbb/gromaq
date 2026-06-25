pub(super) fn blend_rgb8(foreground: [u8; 3], background: [u8; 3], opacity: f32) -> [u8; 3] {
    [
        blend_channel(foreground[0], background[0], opacity),
        blend_channel(foreground[1], background[1], opacity),
        blend_channel(foreground[2], background[2], opacity),
    ]
}

fn blend_channel(foreground: u8, background: u8, opacity: f32) -> u8 {
    ((f32::from(foreground) * opacity) + (f32::from(background) * (1.0 - opacity)))
        .round()
        .clamp(0.0, 255.0) as u8
}
