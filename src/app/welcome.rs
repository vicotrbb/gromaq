//! Default startup welcome screen for the native terminal.

use super::{NativeAppConfig, NativeTerminalRuntimeConfig};
use crate::renderer::RendererConfig;

const AVATAR_WIDTH: usize = 14;
const AVATAR_ROWS: [[u8; 3]; 12] = [
    [45, 55, 77],
    [70, 72, 112],
    [111, 72, 132],
    [159, 82, 133],
    [205, 97, 111],
    [246, 132, 92],
    [244, 174, 96],
    [138, 196, 119],
    [84, 184, 164],
    [88, 154, 198],
    [103, 122, 190],
    [72, 84, 116],
];

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

    let mut text = String::from("\x1b[2J\x1b[H");
    for (row, color) in AVATAR_ROWS.iter().enumerate() {
        text.push_str(&avatar_row(*color));
        text.push_str("  ");
        let (label, value) = &stats[row];
        if !label.is_empty() {
            text.push_str("\x1b[1;38;2;238;244;251m");
            text.push_str(label);
            text.push_str("\x1b[0m");
            text.push_str("  ");
            text.push_str("\x1b[38;2;152;188;219m");
            text.push_str(value);
            text.push_str("\x1b[0m");
        }
        text.push_str("\r\n");
    }
    text
}

fn avatar_row([red, green, blue]: [u8; 3]) -> String {
    format!(
        "\x1b[48;2;{red};{green};{blue}m{}\x1b[0m",
        " ".repeat(AVATAR_WIDTH)
    )
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
        assert!(text.contains("\x1b[48;2;45;55;77m"));
    }
}
