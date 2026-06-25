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
    assert!(text.contains("  [ Gromaq ]"));
    assert!(text.contains("    \x1b[1;38;2;238;244;251mBuild"));
    assert!(text.contains("\x1b[38;2;158;231;255mnative Rust GPU terminal"));
    assert_eq!(text.matches("\r\n").count(), 15);
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
    assert_eq!(lines.len(), 15);
    assert!(lines.iter().all(|line| ansi_visible_width(line) <= 69));
    assert!(text.contains("69x17 cells"));
}

#[test]
fn ansi_visible_width_ignores_color_sequences() {
    assert_eq!(ansi_visible_width("\x1b[38;2;158;231;255mGromaq\x1b[0m"), 6);
}
