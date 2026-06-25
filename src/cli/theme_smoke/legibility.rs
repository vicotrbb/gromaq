use crate::config::{GromaqConfig, format_theme_preset};

use super::super::CliExit;

mod font_metrics;

use font_metrics::has_readable_default_font_metrics;

const FOREGROUND_BACKGROUND_MIN_X100: u64 = 1_200;
const FOREGROUND_SELECTION_MIN_X100: u64 = 800;
const CURSOR_BACKGROUND_MIN_X100: u64 = 700;
const READABLE_ANSI_MIN_X100: u64 = 600;

pub(in crate::cli) fn theme_legibility_smoke_exit() -> CliExit {
    match theme_legibility_report() {
        Ok(report) => CliExit {
            code: 0,
            stdout: format!(
                "theme legibility smoke: ok\npreset: {}\nfont size px: {}\ncell width px: {}\nline height px: {}\nforeground/background contrast x100: {}\nforeground/selection contrast x100: {}\ncursor/background contrast x100: {}\nreadable ansi min contrast x100: {}\n",
                report.preset,
                report.font_size_px,
                report.cell_width_px,
                report.line_height_px,
                report.foreground_background_contrast_x100,
                report.foreground_selection_contrast_x100,
                report.cursor_background_contrast_x100,
                report.readable_ansi_min_contrast_x100
            ),
            stderr: String::new(),
        },
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("theme legibility smoke failed: {error}\n"),
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ThemeLegibilityReport {
    preset: &'static str,
    font_size_px: u16,
    cell_width_px: u16,
    line_height_px: u16,
    foreground_background_contrast_x100: u64,
    foreground_selection_contrast_x100: u64,
    cursor_background_contrast_x100: u64,
    readable_ansi_min_contrast_x100: u64,
}

fn theme_legibility_report() -> Result<ThemeLegibilityReport, String> {
    let config = GromaqConfig::default();
    config.validate().map_err(|error| error.to_string())?;

    let background = config
        .theme
        .background_rgb8()
        .map_err(|error| error.to_string())?;
    let foreground = config
        .theme
        .foreground_rgb8()
        .map_err(|error| error.to_string())?;
    let selection = config
        .theme
        .selection_rgb8()
        .map_err(|error| error.to_string())?;
    let cursor = config
        .theme
        .cursor_rgb8()
        .map_err(|error| error.to_string())?;
    let ansi = config
        .theme
        .ansi_rgb8()
        .map_err(|error| error.to_string())?;

    let readable_ansi_min_contrast_x100 = ansi
        .iter()
        .enumerate()
        .filter(|(index, _)| !matches!(*index, 0 | 8))
        .map(|(_, color)| contrast_ratio_x100(*color, background))
        .min()
        .unwrap_or_default();
    let report = ThemeLegibilityReport {
        preset: format_theme_preset(config.theme.preset),
        font_size_px: config.font.renderer_font_size_px(),
        cell_width_px: config.font.renderer_cell_width_px(),
        line_height_px: config.font.renderer_line_height_px(),
        foreground_background_contrast_x100: contrast_ratio_x100(foreground, background),
        foreground_selection_contrast_x100: contrast_ratio_x100(foreground, selection),
        cursor_background_contrast_x100: contrast_ratio_x100(cursor, background),
        readable_ansi_min_contrast_x100,
    };

    if report.foreground_background_contrast_x100 < FOREGROUND_BACKGROUND_MIN_X100
        || report.foreground_selection_contrast_x100 < FOREGROUND_SELECTION_MIN_X100
        || report.cursor_background_contrast_x100 < CURSOR_BACKGROUND_MIN_X100
        || report.readable_ansi_min_contrast_x100 < READABLE_ANSI_MIN_X100
        || !has_readable_default_font_metrics(
            report.font_size_px,
            report.cell_width_px,
            report.line_height_px,
        )
    {
        return Err(format!("default theme missed legibility gates: {report:?}"));
    }
    Ok(report)
}

fn contrast_ratio_x100(foreground: [u8; 3], background: [u8; 3]) -> u64 {
    (contrast_ratio(foreground, background) * 100.0).round() as u64
}

fn contrast_ratio(foreground: [u8; 3], background: [u8; 3]) -> f64 {
    let foreground = relative_luminance(foreground);
    let background = relative_luminance(background);
    let lighter = foreground.max(background);
    let darker = foreground.min(background);
    (lighter + 0.05) / (darker + 0.05)
}

fn relative_luminance([red, green, blue]: [u8; 3]) -> f64 {
    let [red, green, blue] = [
        srgb_component(red),
        srgb_component(green),
        srgb_component(blue),
    ];
    (0.2126 * red) + (0.7152 * green) + (0.0722 * blue)
}

fn srgb_component(component: u8) -> f64 {
    let value = f64::from(component) / 255.0;
    if value <= 0.03928 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}
