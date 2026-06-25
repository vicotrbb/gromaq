//! Default startup welcome screen for the native terminal.

use super::{NativeAppConfig, NativeTerminalRuntimeConfig};
use crate::renderer::RendererConfig;

const AVATAR_WIDTH: usize = 14;

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
        (
            "Renderer",
            format!(
                "{}px font, {}px line, {}px cell",
                renderer.font_size_px, renderer.line_height_px, renderer.cell_width_px
            ),
        ),
        ("Frame", format!("target {} fps", app.target_fps)),
        (
            "Theme",
            format!(
                "background opacity {}%",
                opacity_percent(renderer.clear_color[3])
            ),
        ),
        (
            "Shell",
            runtime.shell.program.to_string_lossy().into_owned(),
        ),
        ("Font", font_family.to_owned()),
        ("Input", "keyboard, mouse, paste, zoom".to_owned()),
        ("Status", "ready".to_owned()),
        ("", String::new()),
        ("", String::new()),
    ];

    let style = WelcomeStyle::from_renderer(renderer);
    let mut text = String::from("\x1b[2J\x1b[H");
    for (row, color) in style.avatar_rows.iter().enumerate() {
        text.push_str(&avatar_row(*color));
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
    avatar_rows: [[u8; 3]; 12],
}

impl WelcomeStyle {
    fn from_renderer(renderer: &RendererConfig) -> Self {
        let ansi = renderer.ansi_colors_rgb8;
        Self {
            title: renderer.default_foreground_rgb8,
            value: ansi[14],
            avatar_rows: [
                rgba8_to_rgb8(renderer.selection_background_rgba8),
                ansi[4],
                ansi[5],
                ansi[1],
                rgba8_to_rgb8(renderer.cursor_color_rgba8),
                ansi[3],
                ansi[10],
                ansi[2],
                ansi[6],
                ansi[12],
                ansi[13],
                ansi[8],
            ],
        }
    }
}

fn avatar_row([red, green, blue]: [u8; 3]) -> String {
    format!(
        "\x1b[48;2;{red};{green};{blue}m{}\x1b[0m",
        " ".repeat(AVATAR_WIDTH)
    )
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
        assert!(text.contains("34px font, 47px line, 19px cell"));
        assert!(text.contains("background opacity 100%"));
        assert!(text.contains("\x1b[48;2;47;59;82m"));
        assert!(text.contains("\x1b[1;38;2;238;244;251mGromaq"));
        assert!(text.contains("\x1b[38;2;158;231;255mnative Rust GPU terminal"));
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
