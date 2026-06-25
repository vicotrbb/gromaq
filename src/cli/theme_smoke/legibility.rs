use crate::config::{GromaqConfig, format_theme_preset};

use super::super::CliExit;

const FOREGROUND_BACKGROUND_MIN_X100: u64 = 1_200;
const FOREGROUND_SELECTION_MIN_X100: u64 = 800;
const CURSOR_BACKGROUND_MIN_X100: u64 = 700;
const READABLE_ANSI_MIN_X100: u64 = 600;
const DEFAULT_FONT_SIZE_MIN_PX: u16 = 37;
const DEFAULT_CELL_WIDTH_MIN_PX: u16 = 21;
const DEFAULT_LINE_HEIGHT_MIN_PX: u16 = 51;
const DEFAULT_CELL_WIDTH_RATIO_MIN_X100: u64 = 54;
const DEFAULT_CELL_WIDTH_RATIO_MAX_X100: u64 = 62;
const DEFAULT_LINE_HEIGHT_RATIO_MIN_X100: u64 = 130;
const DEFAULT_LINE_HEIGHT_RATIO_MAX_X100: u64 = 145;

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
        || !has_readable_default_font_metrics(&report)
    {
        return Err(format!("default theme missed legibility gates: {report:?}"));
    }
    Ok(report)
}

fn has_readable_default_font_metrics(report: &ThemeLegibilityReport) -> bool {
    if report.font_size_px < DEFAULT_FONT_SIZE_MIN_PX
        || report.cell_width_px < DEFAULT_CELL_WIDTH_MIN_PX
        || report.line_height_px < DEFAULT_LINE_HEIGHT_MIN_PX
    {
        return false;
    }
    let cell_width_ratio_x100 = ratio_x100(report.cell_width_px, report.font_size_px);
    let line_height_ratio_x100 = ratio_x100(report.line_height_px, report.font_size_px);
    (DEFAULT_CELL_WIDTH_RATIO_MIN_X100..=DEFAULT_CELL_WIDTH_RATIO_MAX_X100)
        .contains(&cell_width_ratio_x100)
        && (DEFAULT_LINE_HEIGHT_RATIO_MIN_X100..=DEFAULT_LINE_HEIGHT_RATIO_MAX_X100)
            .contains(&line_height_ratio_x100)
}

fn ratio_x100(numerator: u16, denominator: u16) -> u64 {
    if denominator == 0 {
        return 0;
    }
    ((f64::from(numerator) / f64::from(denominator)) * 100.0).round() as u64
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

#[cfg(test)]
mod tests {
    use super::*;

    fn readable_report() -> ThemeLegibilityReport {
        ThemeLegibilityReport {
            preset: "gromaq-ghostty",
            font_size_px: 37,
            cell_width_px: 21,
            line_height_px: 51,
            foreground_background_contrast_x100: FOREGROUND_BACKGROUND_MIN_X100,
            foreground_selection_contrast_x100: FOREGROUND_SELECTION_MIN_X100,
            cursor_background_contrast_x100: CURSOR_BACKGROUND_MIN_X100,
            readable_ansi_min_contrast_x100: READABLE_ANSI_MIN_X100,
        }
    }

    #[test]
    fn default_font_metrics_gate_accepts_current_readable_defaults() {
        assert!(has_readable_default_font_metrics(&readable_report()));
    }

    #[test]
    fn default_font_metrics_gate_rejects_tiny_defaults() {
        let mut report = readable_report();
        report.font_size_px = 24;
        report.cell_width_px = 13;
        report.line_height_px = 32;

        assert!(!has_readable_default_font_metrics(&report));
    }

    #[test]
    fn default_font_metrics_gate_rejects_cramped_or_loose_geometry() {
        let mut cramped = readable_report();
        cramped.cell_width_px = 16;
        assert!(!has_readable_default_font_metrics(&cramped));

        let mut loose = readable_report();
        loose.line_height_px = 60;
        assert!(!has_readable_default_font_metrics(&loose));
    }
}
