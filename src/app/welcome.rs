//! Default startup welcome screen for the native terminal.

use super::{NativeAppConfig, NativeTerminalRuntimeConfig};
use crate::renderer::RendererConfig;

const AVATAR_EDGE_WIDTH: usize = 4;
const AVATAR_BODY_WIDTH: usize = 10;
const AVATAR_ACCENT_WIDTH: usize = 4;
const SECTION_LABEL: &str = "  --";

pub(super) fn default_welcome_text(
    app: &NativeAppConfig,
    runtime: &NativeTerminalRuntimeConfig,
    renderer: &RendererConfig,
    font_family: &str,
) -> String {
    let stats = [
        WelcomeLine::section("Gromaq"),
        WelcomeLine::metric("Build", "native Rust GPU terminal"),
        WelcomeLine::metric(
            "System",
            format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
        ),
        WelcomeLine::section("Session"),
        WelcomeLine::metric(
            "Terminal",
            format!("{}x{} cells", runtime.terminal_cols, runtime.terminal_rows),
        ),
        WelcomeLine::metric("Scrollback", format!("{} lines", runtime.scrollback_lines)),
        WelcomeLine::metric("Shell", runtime.shell.program.to_string_lossy()),
        WelcomeLine::section("Renderer"),
        WelcomeLine::metric(
            "Font",
            format!(
                "{}  {}px / {}px line",
                font_family, renderer.font_size_px, renderer.line_height_px
            ),
        ),
        WelcomeLine::metric("Cell", format!("{}px wide", renderer.cell_width_px)),
        WelcomeLine::metric(
            "Frame",
            format!("target {} fps, dirty regions", app.target_fps),
        ),
        WelcomeLine::section("Theme"),
        WelcomeLine::metric(
            "Surface",
            format!(
                "{}px padding, opacity {}%",
                renderer.surface_padding_px,
                opacity_percent(renderer.clear_color[3])
            ),
        ),
        WelcomeLine::metric("Palette", "truecolor ANSI + dim text"),
        WelcomeLine::metric("Input", "keyboard, mouse, paste, zoom"),
    ];

    let style = WelcomeStyle::from_renderer(renderer);
    let mut text = String::from("\x1b[2J\x1b[H");
    for (row, avatar) in style.avatar_rows.iter().enumerate() {
        text.push_str(&avatar_row(*avatar));
        text.push_str("  ");
        match &stats[row] {
            WelcomeLine::Metric { label, value } => {
                text.push_str(&bold_foreground(style.title));
                text.push_str(label);
                text.push_str("\x1b[0m");
                text.push_str("  ");
                text.push_str(&foreground(style.value));
                text.push_str(value);
                text.push_str("\x1b[0m");
            }
            WelcomeLine::Section(label) => {
                text.push_str(&foreground(style.section));
                text.push_str(SECTION_LABEL);
                text.push(' ');
                text.push_str(label);
                text.push_str(" --------------------------------");
                text.push_str("\x1b[0m");
            }
        }
        text.push_str("\r\n");
    }
    text
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WelcomeStyle {
    title: [u8; 3],
    value: [u8; 3],
    section: [u8; 3],
    avatar_rows: [AvatarRow; 15],
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WelcomeLine {
    Metric { label: &'static str, value: String },
    Section(&'static str),
}

impl WelcomeLine {
    fn metric(label: &'static str, value: impl ToString) -> Self {
        Self::Metric {
            label,
            value: value.to_string(),
        }
    }

    const fn section(label: &'static str) -> Self {
        Self::Section(label)
    }
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
            section: ansi[8],
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
mod tests;
