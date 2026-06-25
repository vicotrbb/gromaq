//! Default startup welcome screen for the native terminal.

use super::{NativeAppConfig, NativeTerminalRuntimeConfig};
use crate::renderer::RendererConfig;

const AVATAR_EDGE_WIDTH: usize = 4;
const AVATAR_BODY_WIDTH: usize = 10;
const AVATAR_ACCENT_WIDTH: usize = 4;

pub(super) fn default_welcome_text(
    app: &NativeAppConfig,
    runtime: &NativeTerminalRuntimeConfig,
    renderer: &RendererConfig,
    font_family: &str,
) -> String {
    let stats = [
        ("Gromaq", "native Rust GPU terminal".to_owned()),
        (
            "System",
            format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
        ),
        (
            "Terminal",
            format!("{}x{} cells", runtime.terminal_cols, runtime.terminal_rows),
        ),
        ("Scrollback", format!("{} lines", runtime.scrollback_lines)),
        (
            "Renderer",
            format!(
                "{}px font, {}px line, {}px cell",
                renderer.font_size_px, renderer.line_height_px, renderer.cell_width_px
            ),
        ),
        ("Frame", format!("target {} fps", app.target_fps)),
        (
            "Surface",
            format!(
                "{}px padding, {}px spacing",
                renderer.surface_padding_px, renderer.cell_spacing_px
            ),
        ),
        (
            "Theme",
            format!(
                "background opacity {}%",
                opacity_percent(renderer.clear_color[3])
            ),
        ),
        ("Palette", "truecolor ANSI + dim text".to_owned()),
        (
            "Shell",
            runtime.shell.program.to_string_lossy().into_owned(),
        ),
        ("Font", font_family.to_owned()),
        ("Input", "keyboard, mouse, paste, zoom".to_owned()),
        ("Clipboard", "native copy/paste + OSC 52".to_owned()),
        ("Status", "ready".to_owned()),
        ("", String::new()),
        ("", String::new()),
    ];

    let style = WelcomeStyle::from_renderer(renderer);
    let mut text = String::from("\x1b[2J\x1b[H");
    for (row, avatar) in style.avatar_rows.iter().enumerate() {
        text.push_str(&avatar_row(*avatar));
        text.push_str("  ");
        let (label, value) = &stats[row];
        if !label.is_empty() {
            text.push_str(&bold_foreground(style.title));
            text.push_str(label);
            text.push_str("\x1b[0m");
            text.push_str("  ");
            text.push_str(&foreground(style.value));
            text.push_str(value);
            text.push_str("\x1b[0m");
        }
        text.push_str("\r\n");
    }
    text
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WelcomeStyle {
    title: [u8; 3],
    value: [u8; 3],
    avatar_rows: [AvatarRow; 16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AvatarRow {
    edge: [u8; 3],
    body: [u8; 3],
    accent: [u8; 3],
}

impl AvatarRow {
    const fn new(edge: [u8; 3], body: [u8; 3], accent: [u8; 3]) -> Self {
        Self { edge, body, accent }
    }
}

impl WelcomeStyle {
    fn from_renderer(renderer: &RendererConfig) -> Self {
        let ansi = renderer.ansi_colors_rgb8;
        let base = ansi[0];
        let muted = ansi[8];
        let selection = rgba8_to_rgb8(renderer.selection_background_rgba8);
        let cursor = rgba8_to_rgb8(renderer.cursor_color_rgba8);
        Self {
            title: renderer.default_foreground_rgb8,
            value: ansi[14],
            avatar_rows: [
                AvatarRow::new(base, selection, ansi[4]),
                AvatarRow::new(base, ansi[4], ansi[12]),
                AvatarRow::new(selection, ansi[4], ansi[13]),
                AvatarRow::new(selection, ansi[5], ansi[1]),
                AvatarRow::new(ansi[4], ansi[5], cursor),
                AvatarRow::new(ansi[5], ansi[1], cursor),
                AvatarRow::new(ansi[1], cursor, ansi[3]),
                AvatarRow::new(cursor, ansi[3], ansi[11]),
                AvatarRow::new(ansi[3], ansi[10], ansi[2]),
                AvatarRow::new(ansi[10], ansi[2], ansi[6]),
                AvatarRow::new(ansi[2], ansi[6], ansi[14]),
                AvatarRow::new(ansi[6], ansi[12], ansi[14]),
                AvatarRow::new(ansi[12], ansi[13], ansi[5]),
                AvatarRow::new(ansi[13], muted, ansi[8]),
                AvatarRow::new(muted, selection, base),
                AvatarRow::new(base, muted, selection),
            ],
        }
    }
}

fn avatar_row(row: AvatarRow) -> String {
    format!(
        "{}{}{}\x1b[0m",
        background_segment(row.edge, AVATAR_EDGE_WIDTH),
        background_segment(row.body, AVATAR_BODY_WIDTH),
        background_segment(row.accent, AVATAR_ACCENT_WIDTH)
    )
}

fn background_segment([red, green, blue]: [u8; 3], width: usize) -> String {
    format!("\x1b[48;2;{red};{green};{blue}m{}", " ".repeat(width))
}

fn foreground([red, green, blue]: [u8; 3]) -> String {
    format!("\x1b[38;2;{red};{green};{blue}m")
}

fn bold_foreground([red, green, blue]: [u8; 3]) -> String {
    format!("\x1b[1;38;2;{red};{green};{blue}m")
}

fn rgba8_to_rgb8([red, green, blue, _alpha]: [u8; 4]) -> [u8; 3] {
    [red, green, blue]
}

fn opacity_percent(opacity: f64) -> u32 {
    (opacity.clamp(0.0, 1.0) * 100.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_welcome_text_reports_terminal_and_renderer_stats() {
        let text = default_welcome_text(
            &NativeAppConfig::default(),
            &NativeTerminalRuntimeConfig::default(),
            &RendererConfig::default(),
            "monospace",
        );

        assert!(text.contains("Gromaq"));
        assert!(text.contains("native Rust GPU terminal"));
        assert!(text.contains("120x36 cells"));
        assert!(text.contains("10000 lines"));
        assert!(text.contains("32px font, 44px line, 18px cell"));
        assert!(text.contains("14px padding, 0px spacing"));
        assert!(text.contains("background opacity 100%"));
        assert!(text.contains("truecolor ANSI + dim text"));
        assert!(text.contains("native copy/paste + OSC 52"));
        assert!(text.contains("\x1b[48;2;47;59;82m"));
        assert!(text.contains("\x1b[1;38;2;238;244;251mGromaq"));
        assert!(text.contains("\x1b[38;2;158;231;255mnative Rust GPU terminal"));
        assert_eq!(text.matches("\r\n").count(), 16);
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
            "monospace",
        );

        assert!(text.contains("\x1b[48;2;7;8;9m"));
        assert!(text.contains("\x1b[1;38;2;1;2;3mGromaq"));
        assert!(text.contains("\x1b[38;2;10;11;12mnative Rust GPU terminal"));
    }
}
