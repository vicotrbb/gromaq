use super::layout::ansi_visible_width;
use super::*;

use std::collections::HashSet;

mod support;

use support::{avatar_sgr_rgb_colors, contrast_ratio, is_terminal_block, strip_ansi};

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
    assert!(text.contains("JetBrains Mono Nerd Font 32/44px"));
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
fn default_welcome_avatar_is_trimmed_and_uses_supported_terminal_glyphs() {
    let lines: Vec<_> = WELCOME_AVATAR_ANSI.lines().collect();
    let widths: Vec<_> = lines.iter().map(|line| ansi_visible_width(line)).collect();
    let max_width = widths.iter().copied().max().unwrap_or(0);

    assert_eq!(lines.len(), 17);
    assert_eq!(max_width, 33);
    assert!(widths.iter().all(|width| *width == 33));
    assert!(WELCOME_AVATAR_ANSI.chars().any(is_terminal_block));

    // The avatar is baked for the default gromaq cell (18x44px). A near-square
    // source must render wider-than-tall in cell counts to avoid the vertical
    // stretching that left the old avatar 39% too narrow. The 17-row avatar
    // spends one extra row on vertical detail while keeping the ratio safely
    // above the old stretched output.
    let ratio = max_width as f64 / lines.len() as f64;
    assert!(ratio >= 1.9, "avatar aspect ratio regressed: {ratio}");
}

#[test]
fn default_welcome_avatar_keeps_dense_color_detail() {
    let colors = avatar_sgr_rgb_colors(WELCOME_AVATAR_ANSI);
    let unique_colors: HashSet<_> = colors.iter().copied().collect();

    assert!(
        colors.len() >= 260,
        "avatar foreground detail too sparse: {} colored cells",
        colors.len()
    );
    assert!(
        unique_colors.len() >= 250,
        "avatar color detail too low: {} unique foreground colors",
        unique_colors.len()
    );
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
    assert!(text.contains("native Rust GPU terminal"));
}

#[test]
fn default_welcome_preview_width_keeps_font_metric_complete() {
    let runtime = NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 18,
        ..NativeTerminalRuntimeConfig::default()
    };
    let text = default_welcome_text(
        &NativeAppConfig::default(),
        &runtime,
        &RendererConfig::default(),
        "JetBrains Mono Nerd Font",
    );

    let raw_font_line = text
        .split("\r\n")
        .find(|line| line.contains("Font"))
        .expect("welcome text must include a Font metric row");
    let font_line = strip_ansi(raw_font_line);

    assert!(
        font_line.contains("JetBrains Mono Nerd Font 32/44px"),
        "font metric was clipped or too verbose: {font_line}"
    );
    assert!(
        ansi_visible_width(raw_font_line) <= 80,
        "font metric row exceeded preview width"
    );
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
