//! Default startup welcome screen for the native terminal.

use super::{NativeAppConfig, NativeTerminalRuntimeConfig};
use crate::renderer::RendererConfig;

const WELCOME_AVATAR_ANSI: &str = include_str!("../../images/avatar/avatar-welcome.ansi");

mod layout;

use layout::{WelcomeEntry, WelcomeStyle, push_welcome_row};

pub(crate) fn default_welcome_text(
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
    for (row, avatar) in WELCOME_AVATAR_ANSI.lines().enumerate() {
        let entry = match &stats[row] {
            WelcomeLine::Metric { label, value } => WelcomeEntry::Metric { label, value },
            WelcomeLine::Section(label) => WelcomeEntry::Section(label),
        };
        push_welcome_row(&mut text, runtime.terminal_cols, avatar, entry, style);
    }
    text
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

impl WelcomeStyle {
    fn from_renderer(renderer: &RendererConfig) -> Self {
        let ansi = renderer.ansi_colors_rgb8;
        Self {
            title: renderer.default_foreground_rgb8,
            value: ansi[14],
            section: ansi[8],
        }
    }
}

fn opacity_percent(opacity: f64) -> u32 {
    (opacity.clamp(0.0, 1.0) * 100.0).round() as u32
}

#[cfg(test)]
mod tests;
