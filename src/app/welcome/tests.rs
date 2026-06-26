use super::layout::ansi_visible_width;
use super::*;

#[test]
fn default_welcome_text_reports_terminal_and_renderer_stats() {
    let text = default_welcome_text(
        &NativeAppConfig::default(),
        &NativeTerminalRuntimeConfig::default(),
        &RendererConfig::default(),
        "JetBrains Mono Nerd Font",
    );

    assert!(text.contains("[ Gromaq ]"));
    assert!(text.contains("[ Session ]"));
    assert!(text.contains("[ Renderer ]"));
    assert!(text.contains("[ Theme ]"));
    assert!(text.contains("native Rust GPU terminal"));
    assert!(text.contains("120x36 cells"));
    assert!(text.contains("10000 lines"));
    assert!(text.contains("JetBrains Mono Nerd Font  32px / 44px line"));
    assert!(text.contains("18px wide"));
    assert!(text.contains("14px padding, opacity 100%"));
    assert!(text.contains("truecolor ANSI + dim text"));
    assert!(text.contains(WELCOME_AVATAR_ANSI.lines().nth(2).unwrap()));
    assert!(!text.contains("GMQ"));
    assert!(!text.contains("TERMINAL"));
    assert!(!text.contains("REBORN"));
    assert!(text.contains("  [ Gromaq ]"));
    assert!(text.contains("    \x1b[1;38;2;238;244;251mBuild"));
    assert!(text.contains("\x1b[38;2;158;231;255mnative Rust GPU terminal"));
    assert_eq!(
        text.matches("\r\n").count(),
        WELCOME_AVATAR_ANSI.lines().count()
    );
}

#[test]
fn default_welcome_avatar_is_trimmed_and_uses_supported_block_glyphs() {
    let lines: Vec<_> = WELCOME_AVATAR_ANSI.lines().collect();
    let widths: Vec<_> = lines.iter().map(|line| ansi_visible_width(line)).collect();
    let max_width = widths.iter().copied().max().unwrap_or(0);

    assert_eq!(lines.len(), 15);
    assert_eq!(max_width, 32);
    assert!(widths.iter().all(|width| *width == 32));
    assert!(WELCOME_AVATAR_ANSI.chars().any(is_half_block));

    // The avatar is baked for the default gromaq cell (18x44px). A near-square
    // source must render wider-than-tall in cell counts (ratio ~2.3) to avoid
    // the vertical stretching that left the old avatar 39% too narrow. This
    // guards the aspect-correct half-block rendering against regressions.
    let ratio = max_width as f64 / lines.len() as f64;
    assert!(ratio >= 2.0, "avatar aspect ratio regressed: {ratio}");
}

#[test]
fn default_welcome_text_uses_renderer_theme_colors() {
    let mut renderer = RendererConfig {
        default_foreground_rgb8: [1, 2, 3],
        cursor_color_rgba8: [4, 5, 6, 255],
        selection_background_rgba8: [7, 8, 9, 255],
        ..RendererConfig::default()
    };
    renderer.ansi_colors_rgb8[14] = [10, 11, 12];
    let text = default_welcome_text(
        &NativeAppConfig::default(),
        &NativeTerminalRuntimeConfig::default(),
        &renderer,
        "JetBrains Mono Nerd Font",
    );

    assert!(text.contains(WELCOME_AVATAR_ANSI.lines().nth(2).unwrap()));
    assert!(text.contains("\x1b[1;38;2;1;2;3mBuild"));
    assert!(text.contains("\x1b[38;2;10;11;12mnative Rust GPU terminal"));
}

fn is_half_block(ch: char) -> bool {
    matches!(ch, '▀' | '▄' | '█')
}

#[test]
fn default_welcome_text_does_not_wrap_at_narrow_runtime_width() {
    let runtime = NativeTerminalRuntimeConfig {
        terminal_cols: 69,
        terminal_rows: 17,
        ..NativeTerminalRuntimeConfig::default()
    };
    let text = default_welcome_text(
        &NativeAppConfig::default(),
        &runtime,
        &RendererConfig::default(),
        "JetBrains Mono Nerd Font",
    );

    let lines: Vec<_> = text.split("\r\n").filter(|line| !line.is_empty()).collect();
    assert_eq!(lines.len(), WELCOME_AVATAR_ANSI.lines().count());
    assert!(lines.iter().all(|line| ansi_visible_width(line) <= 69));
    assert!(text.contains("69x17 cells"));
}

#[test]
fn ansi_visible_width_ignores_color_sequences() {
    assert_eq!(ansi_visible_width("\x1b[38;2;158;231;255mGromaq\x1b[0m"), 6);
}

#[test]
fn default_welcome_avatar_keeps_readable_contrast_on_dark_background() {
    // The avatar is baked for the default gromaq-ghostty background (#101216).
    // Guard the luminance floor added to boostTerminalColor so the avatar cannot
    // silently regress to the muddy low-contrast output (min 1.13:1, 83% below
    // 3:1) that left it nearly invisible on the dark background.
    const BACKGROUND: [u8; 3] = [0x10, 0x12, 0x16];
    const MIN_CONTRAST: f64 = 3.0;

    let ratios: Vec<f64> = avatar_sgr_rgb_colors(WELCOME_AVATAR_ANSI)
        .into_iter()
        .map(|color| contrast_ratio(color, BACKGROUND))
        .collect();

    assert!(!ratios.is_empty(), "avatar must define truecolor cells");
    let mean = ratios.iter().sum::<f64>() / ratios.len() as f64;
    let readable = ratios
        .iter()
        .filter(|&&ratio| ratio >= MIN_CONTRAST)
        .count();
    assert!(
        mean >= MIN_CONTRAST,
        "avatar mean contrast {mean:.2} below {MIN_CONTRAST}:1"
    );
    assert!(
        readable * 100 >= ratios.len() * 85,
        "only {readable}/{} avatar cells reach {MIN_CONTRAST}:1",
        ratios.len()
    );
}

fn avatar_sgr_rgb_colors(ansi: &str) -> Vec<[u8; 3]> {
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

fn contrast_ratio(foreground: [u8; 3], background: [u8; 3]) -> f64 {
    let foreground_luma = relative_luminance(foreground);
    let background_luma = relative_luminance(background);
    let lighter = foreground_luma.max(background_luma);
    let darker = foreground_luma.min(background_luma);
    (lighter + 0.05) / (darker + 0.05)
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
