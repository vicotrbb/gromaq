use super::layout::ansi_visible_width;
use super::*;

use std::ffi::OsString;

mod avatar;
mod support;

use support::strip_ansi;

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
    assert!(text.contains("tmux Cmd/Ctrl+Shift+T"));
    assert!(text.contains(WELCOME_AVATAR_ANSI.lines().nth(2).unwrap()));
    assert!(!text.contains("GMQ"));
    assert!(!text.contains("TERMINAL"));
    assert!(!text.contains("REBORN"));
    assert!(text.contains("  [ Gromaq ]"));
    assert!(text.contains("  \x1b[1;38;2;238;244;251mBuild"));
    assert!(text.contains("\x1b[38;2;158;231;255mnative Rust GPU terminal"));
    assert_eq!(
        text.matches("\r\n").count(),
        WELCOME_AVATAR_ANSI.lines().count()
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
fn default_welcome_text_marks_clipped_metric_values() {
    let long_shell =
        "/very/long/path/to/custom/developer/shell/program/gromaqshell-with-extra-flags";
    let runtime = NativeTerminalRuntimeConfig {
        terminal_cols: 69,
        terminal_rows: 17,
        shell: crate::pty::ShellCommand {
            program: OsString::from(long_shell),
            args: Vec::new(),
            cwd: None,
        },
        ..NativeTerminalRuntimeConfig::default()
    };
    let text = default_welcome_text(
        &NativeAppConfig::default(),
        &runtime,
        &RendererConfig::default(),
        "JetBrains Mono Nerd Font",
    );

    let raw_shell_line = text
        .split("\r\n")
        .find(|line| line.contains("Shell"))
        .expect("welcome text must include a Shell metric row");
    let shell_line = strip_ansi(raw_shell_line);

    assert!(
        shell_line.contains("..."),
        "clipped shell metric should show a truncation marker: {shell_line}"
    );
    assert!(
        !shell_line.contains(long_shell),
        "long shell metric should be clipped: {shell_line}"
    );
    assert!(
        ansi_visible_width(raw_shell_line) <= 69,
        "shell metric row exceeded preview width"
    );
}

#[test]
fn ansi_visible_width_ignores_color_sequences() {
    assert_eq!(ansi_visible_width("\x1b[38;2;158;231;255mGromaq\x1b[0m"), 6);
}
