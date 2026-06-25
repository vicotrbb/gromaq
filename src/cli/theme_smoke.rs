//! Theme and default text legibility CLI smoke command.

use std::fs;
use std::path::Path;

use crate::app::load_default_native_glyph_cache;
use crate::config::{GromaqConfig, format_theme_preset};
use crate::renderer::{
    GlyphAtlas, GlyphAtlasConfig, PreparedSurfaceGlyphFrame, RenderPlanner, RendererConfig,
};
use crate::selection::SelectionRange;
use crate::{Terminal, TerminalConfig};

use super::CliExit;

const THEME_PREVIEW_COLS: u16 = 56;
const THEME_PREVIEW_ROWS: u16 = 6;
const FOREGROUND_BACKGROUND_MIN_X100: u64 = 1_200;
const FOREGROUND_SELECTION_MIN_X100: u64 = 800;
const CURSOR_BACKGROUND_MIN_X100: u64 = 700;
const READABLE_ANSI_MIN_X100: u64 = 600;
const THEME_PREVIEW_TEXT: &str = "\
\x1b[1mGromaq\x1b[0m native terminal preview\r\n\
\x1b[31merror\x1b[0m  \x1b[32mok\x1b[0m  \x1b[33mwarn\x1b[0m  \x1b[34mblue\x1b[0m  \x1b[35mmagenta\x1b[0m\r\n\
~/Daedalus/gromaq > cargo test --all\r\n\
output stays readable after prompt repaint\r\n";

pub(super) fn theme_legibility_smoke_exit() -> CliExit {
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

pub(super) fn theme_preview_snapshot_exit(path: &str) -> CliExit {
    match theme_preview_snapshot_report(path) {
        Ok(report) => CliExit {
            code: 0,
            stdout: format!(
                "theme preview snapshot: ok\npath: {path}\nbytes written: {}\nframe size: {}x{}\npreview pixels: {}\nfont size px: {}\ncell width px: {}\nline height px: {}\nsurface padding px: {}\nprepared quads: {}\nbackground quads: {}\ncursor quads: {}\natlas bytes: {}\n",
                report.bytes_written,
                report.width,
                report.height,
                report.preview_pixels,
                report.font_size_px,
                report.cell_width_px,
                report.line_height_px,
                report.surface_padding_px,
                report.prepared_quads,
                report.background_quads,
                report.cursor_quads,
                report.atlas_bytes
            ),
            stderr: String::new(),
        },
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("theme preview snapshot failed: {error}\n"),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ThemePreviewSnapshotReport {
    bytes_written: usize,
    width: u32,
    height: u32,
    preview_pixels: usize,
    font_size_px: u16,
    cell_width_px: u16,
    line_height_px: u16,
    surface_padding_px: u16,
    prepared_quads: usize,
    background_quads: usize,
    cursor_quads: usize,
    atlas_bytes: usize,
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
    {
        return Err(format!("default theme missed legibility gates: {report:?}"));
    }
    Ok(report)
}

fn theme_preview_snapshot_report(path: &str) -> Result<ThemePreviewSnapshotReport, String> {
    let renderer_config = RendererConfig::default();
    let mut terminal = Terminal::new(
        TerminalConfig::new(THEME_PREVIEW_COLS, THEME_PREVIEW_ROWS)
            .map_err(|error| error.to_string())?,
    );
    terminal
        .write_str(THEME_PREVIEW_TEXT)
        .map_err(|error| error.to_string())?;
    terminal.set_selection(SelectionRange::new((2, 0), (2, 16)));
    let dirty_regions = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(512).map_err(|error| error.to_string())?);
    let mut planner = RenderPlanner::with_visual_theme(
        renderer_config.font_size_px,
        renderer_config.default_foreground_rgb8,
        renderer_config.ansi_colors_rgb8,
        renderer_config.selection_background_rgba8,
        renderer_config.dim_opacity,
    );
    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty_regions,
            &mut atlas,
        )
        .map_err(|error| error.to_string())?;
    if plan.glyphs.len() < 24 {
        return Err(format!(
            "theme preview rendered too few glyphs: {}",
            plan.glyphs.len()
        ));
    }
    if plan.backgrounds.is_empty() {
        return Err("theme preview did not include a selection background".to_owned());
    }
    let mut glyph_cache = load_default_native_glyph_cache().map_err(|error| error.to_string())?;
    let glyphs = glyph_cache
        .rasterize_plan(&plan)
        .map_err(|error| error.to_string())?;
    let prepared = PreparedSurfaceGlyphFrame::from_render_plan(
        &plan,
        &glyphs.bitmaps,
        renderer_config.cell_width_px,
        renderer_config.line_height_px,
        renderer_config.clear_color,
        renderer_config.cursor_color_rgba8,
        renderer_config.surface_padding_px,
    )
    .map_err(|error| error.to_string())?;
    let preview = prepared
        .preview_rgba8()
        .map_err(|error| error.to_string())?;
    validate_theme_preview_background(&preview.rgba, preview.width, renderer_config.clear_color)?;
    let snapshot = theme_preview_ppm_bytes(preview.width, preview.height, &preview.rgba)?;
    fs::write(Path::new(path), &snapshot)
        .map_err(|error| format!("failed to write theme preview snapshot: {error}"))?;
    let frame = prepared.as_surface_glyph_frame();
    Ok(ThemePreviewSnapshotReport {
        bytes_written: snapshot.len(),
        width: preview.width,
        height: preview.height,
        preview_pixels: preview.rgba.len() / 4,
        font_size_px: renderer_config.font_size_px,
        cell_width_px: renderer_config.cell_width_px,
        line_height_px: renderer_config.line_height_px,
        surface_padding_px: renderer_config.surface_padding_px,
        prepared_quads: frame.batch.quads.len(),
        background_quads: frame.background_batch.quads.len(),
        cursor_quads: frame.cursor_batch.quads.len(),
        atlas_bytes: frame.atlas.rgba.len(),
    })
}

fn contrast_ratio_x100(foreground: [u8; 3], background: [u8; 3]) -> u64 {
    (contrast_ratio(foreground, background) * 100.0).round() as u64
}

fn validate_theme_preview_background(
    pixels: &[u8],
    width: u32,
    clear_color: [f64; 4],
) -> Result<(), String> {
    let expected = linear_f64_rgba_to_srgb8(clear_color);
    let Some(pixel) = pixels.get(0..4) else {
        return Err("theme preview did not contain any pixels".to_owned());
    };
    if pixel != expected {
        return Err(format!(
            "theme preview background pixel {:?} did not match expected {:?} at width {width}",
            pixel, expected
        ));
    }
    Ok(())
}

fn theme_preview_ppm_bytes(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>, String> {
    let expected_rgba_len = usize::try_from(u64::from(width) * u64::from(height) * 4)
        .map_err(|_| "theme preview snapshot is too large".to_owned())?;
    if pixels.len() != expected_rgba_len {
        return Err(format!(
            "theme preview snapshot expected {expected_rgba_len} RGBA bytes, got {}",
            pixels.len()
        ));
    }
    let rgb_len = usize::try_from(u64::from(width) * u64::from(height) * 3)
        .map_err(|_| "theme preview snapshot RGB buffer is too large".to_owned())?;
    let header = format!("P6\n{width} {height}\n255\n");
    let mut snapshot = Vec::new();
    snapshot
        .try_reserve_exact(header.len() + rgb_len)
        .map_err(|_| "theme preview snapshot allocation failed".to_owned())?;
    snapshot.extend_from_slice(header.as_bytes());
    for pixel in pixels.chunks_exact(4) {
        snapshot.extend_from_slice(&pixel[..3]);
    }
    Ok(snapshot)
}

fn linear_f64_rgba_to_srgb8([red, green, blue, alpha]: [f64; 4]) -> [u8; 4] {
    [
        linear_channel_to_srgb8(red),
        linear_channel_to_srgb8(green),
        linear_channel_to_srgb8(blue),
        linear_channel_to_srgb8(alpha),
    ]
}

fn linear_channel_to_srgb8(value: f64) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let srgb = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        (1.055 * value.powf(1.0 / 2.4)) - 0.055
    };
    (srgb * 255.0).round().clamp(0.0, 255.0) as u8
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
