pub(super) fn is_terminal_block(ch: char) -> bool {
    "▀▄█▘▝▖▗▌▐▚▞▛▜▙▟".contains(ch)
}

pub(super) fn strip_ansi(value: &str) -> String {
    let mut stripped = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            stripped.push(ch);
        }
    }
    stripped
}

pub(super) fn avatar_sgr_rgb_colors(ansi: &str) -> Vec<[u8; 3]> {
    let bytes = ansi.as_bytes();
    let mut colors = Vec::new();
    let mut i = 0;
    while i + 7 <= bytes.len() {
        let is_truecolor_sgr = bytes[i] == 0x1b
            && bytes[i + 1] == b'['
            && (bytes[i + 2] == b'3' || bytes[i + 2] == b'4')
            && bytes[i + 3] == b'8'
            && bytes[i + 4] == b';'
            && bytes[i + 5] == b'2'
            && bytes[i + 6] == b';';
        if is_truecolor_sgr && let Some((color, consumed)) = parse_sgr_rgb(&bytes[i + 7..]) {
            colors.push(color);
            i += 7 + consumed;
            continue;
        }
        i += 1;
    }
    colors
}

pub(super) fn contrast_ratio(foreground: [u8; 3], background: [u8; 3]) -> f64 {
    let foreground_luma = relative_luminance(foreground);
    let background_luma = relative_luminance(background);
    let lighter = foreground_luma.max(background_luma);
    let darker = foreground_luma.min(background_luma);
    (lighter + 0.05) / (darker + 0.05)
}

fn parse_sgr_rgb(slice: &[u8]) -> Option<([u8; 3], usize)> {
    let mut color = [0u8; 3];
    let mut pos = 0;
    for slot in color.iter_mut() {
        let mut value = 0u32;
        let mut digits = 0;
        while pos < slice.len() && slice[pos].is_ascii_digit() {
            value = value * 10 + u32::from(slice[pos] - b'0');
            pos += 1;
            digits += 1;
        }
        if digits == 0 || value > 255 {
            return None;
        }
        *slot = value as u8;
        if pos < slice.len() && slice[pos] == b';' {
            pos += 1;
        }
    }
    (pos < slice.len() && slice[pos] == b'm').then_some((color, pos + 1))
}

fn relative_luminance([red, green, blue]: [u8; 3]) -> f64 {
    let channel = |value: u8| {
        let c = f64::from(value) / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(red) + 0.7152 * channel(green) + 0.0722 * channel(blue)
}
