//! Default startup welcome screen for the native terminal.

use super::{NativeAppConfig, NativeTerminalRuntimeConfig};
use crate::renderer::RendererConfig;

const SECTION_RULE_WIDTH: usize = 36;
const WELCOME_AVATAR_ANSI: &str = include_str!("../../images/avatar/avatar-welcome.ansi");
const WELCOME_GAP_WIDTH: usize = 2;

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
        text.push_str(avatar);
        let stat_budget = stat_budget(runtime.terminal_cols, avatar);
        if stat_budget == 0 {
            text.push_str("\r\n");
            continue;
        }
        text.push_str(&" ".repeat(WELCOME_GAP_WIDTH.min(stat_budget)));
        let stat_budget = stat_budget.saturating_sub(WELCOME_GAP_WIDTH);
        let stat = match &stats[row] {
            WelcomeLine::Metric { label, value } => metric_line(style, label, value, stat_budget),
            WelcomeLine::Section(label) => section_line(style, label, stat_budget),
        };
        push_ansi_clipped(&mut text, &stat, stat_budget);
        text.push_str("\r\n");
    }
    text
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WelcomeStyle {
    title: [u8; 3],
    value: [u8; 3],
    section: [u8; 3],
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

fn stat_budget(cols: u16, avatar: &str) -> usize {
    usize::from(cols).saturating_sub(ansi_visible_width(avatar))
}

fn metric_line(style: WelcomeStyle, label: &str, value: &str, max_width: usize) -> String {
    let prefix_width = 4 + label.len() + 2;
    let value = clipped_plain(value, max_width.saturating_sub(prefix_width));
    format!(
        "    {}{label}\x1b[0m  {}{value}\x1b[0m",
        bold_foreground(style.title),
        foreground(style.value)
    )
}

fn section_line(style: WelcomeStyle, label: &str, max_width: usize) -> String {
    format!(
        "{}{}\x1b[0m",
        foreground(style.section),
        section_header(label, max_width)
    )
}

fn section_header(label: &str, max_width: usize) -> String {
    let title = format!("  [ {label} ] ");
    if max_width <= title.len() {
        return clipped_plain(&title, max_width);
    }
    let rule_width = SECTION_RULE_WIDTH
        .min(max_width)
        .saturating_sub(title.len())
        .max(4);
    format!("{title}{}", "-".repeat(rule_width))
}

fn push_ansi_clipped(output: &mut String, value: &str, max_width: usize) {
    let mut visible = 0;
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            output.push(ch);
            for next in chars.by_ref() {
                output.push(next);
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        if visible >= max_width {
            break;
        }
        output.push(ch);
        visible += 1;
    }
    output.push_str("\x1b[0m");
}

fn clipped_plain(value: &str, max_width: usize) -> String {
    value.chars().take(max_width).collect()
}

fn ansi_visible_width(value: &str) -> usize {
    let mut width = 0;
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            width += 1;
        }
    }
    width
}

fn foreground([red, green, blue]: [u8; 3]) -> String {
    format!("\x1b[38;2;{red};{green};{blue}m")
}

fn bold_foreground([red, green, blue]: [u8; 3]) -> String {
    format!("\x1b[1;38;2;{red};{green};{blue}m")
}

fn opacity_percent(opacity: f64) -> u32 {
    (opacity.clamp(0.0, 1.0) * 100.0).round() as u32
}

#[cfg(test)]
mod tests;
