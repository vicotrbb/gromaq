use std::fs;
use std::path::Path;

use crate::app::load_default_native_glyph_cache;
use crate::config::{GromaqConfig, format_theme_preset};
use crate::renderer::{
    GlyphAtlas, GlyphAtlasConfig, PreparedSurfaceGlyphFrame, PreparedSurfaceGlyphFrameConfig,
    RenderPlanner, RendererConfig,
};
use crate::selection::SelectionRange;
use crate::{Terminal, TerminalConfig};

use super::super::CliExit;

mod output;

use super::pixels::validate_theme_preview_pixels;
use super::ppm::ppm_bytes;
use output::{
    ThemePreviewSnapshotReport, theme_preview_snapshot_error, theme_preview_snapshot_success,
};

const THEME_PREVIEW_COLS: u16 = 56;
const THEME_PREVIEW_ROWS: u16 = 6;
const THEME_PREVIEW_TEXT: &str = "\
\x1b[1mGromaq\x1b[0m native terminal preview\r\n\
\x1b[31merror\x1b[0m  \x1b[32mok\x1b[0m  \x1b[33mwarn\x1b[0m  \x1b[34mblue\x1b[0m  \x1b[35mmagenta\x1b[0m\r\n\
~/Daedalus/gromaq > cargo test --all\r\n\
output stays readable after prompt repaint\r\n";

pub(in crate::cli) fn theme_preview_snapshot_exit(path: &str) -> CliExit {
    match theme_preview_snapshot_report(&GromaqConfig::default(), path) {
        Ok(report) => theme_preview_snapshot_success(path, &report),
        Err(error) => theme_preview_snapshot_error(error),
    }
}

pub(in crate::cli) fn theme_preview_config_exit(config_path: &str, snapshot_path: &str) -> CliExit {
    match GromaqConfig::from_toml_file(config_path)
        .map_err(|error| error.to_string())
        .and_then(|config| theme_preview_snapshot_report(&config, snapshot_path))
    {
        Ok(report) => theme_preview_snapshot_success(snapshot_path, &report),
        Err(error) => theme_preview_snapshot_error(error),
    }
}

fn theme_preview_snapshot_report(
    config: &GromaqConfig,
    path: &str,
) -> Result<ThemePreviewSnapshotReport, String> {
    let renderer_config = RendererConfig::from_gromaq_config(config)
        .map_err(|error| format!("failed to build renderer config: {error}"))?;
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
        PreparedSurfaceGlyphFrameConfig {
            cell_width_px: renderer_config.cell_width_px,
            line_height_px: renderer_config.line_height_px,
            clear_color: renderer_config.clear_color,
            cursor_color_rgba8: renderer_config.cursor_color_rgba8,
            surface_padding_px: renderer_config.surface_padding_px,
            cell_spacing_px: renderer_config.cell_spacing_px,
        },
    )
    .map_err(|error| error.to_string())?;
    let preview = prepared
        .preview_rgba8()
        .map_err(|error| error.to_string())?;
    let pixel_report = validate_theme_preview_pixels(
        &preview.rgba,
        preview.width,
        renderer_config.clear_color,
        renderer_config.selection_background_rgba8,
        renderer_config.cursor_color_rgba8,
    )?;
    let snapshot = ppm_bytes(preview.width, preview.height, &preview.rgba)?;
    fs::write(Path::new(path), &snapshot)
        .map_err(|error| format!("failed to write theme preview snapshot: {error}"))?;
    let frame = prepared.as_surface_glyph_frame();
    Ok(ThemePreviewSnapshotReport {
        preset: format_theme_preset(config.theme.preset),
        bytes_written: snapshot.len(),
        width: preview.width,
        height: preview.height,
        preview_pixels: preview.rgba.len() / 4,
        font_size_px: renderer_config.font_size_px,
        cell_width_px: renderer_config.cell_width_px,
        line_height_px: renderer_config.line_height_px,
        background_opacity_percent: opacity_percent(renderer_config.clear_color[3]),
        cursor_opacity_percent: opacity_percent_from_alpha(renderer_config.cursor_color_rgba8[3]),
        selection_opacity_percent: opacity_percent_from_alpha(
            renderer_config.selection_background_rgba8[3],
        ),
        surface_padding_px: renderer_config.surface_padding_px,
        cell_spacing_px: renderer_config.cell_spacing_px,
        high_contrast_text_pixels: pixel_report.high_contrast_text_pixels,
        selection_pixels: pixel_report.selection_pixels,
        cursor_pixels: pixel_report.cursor_pixels,
        prepared_quads: frame.batch.quads.len(),
        background_quads: frame.background_batch.quads.len(),
        cursor_quads: frame.cursor_batch.quads.len(),
        atlas_bytes: frame.atlas.rgba.len(),
    })
}

fn opacity_percent(opacity: f64) -> u32 {
    (opacity.clamp(0.0, 1.0) * 100.0).round() as u32
}

fn opacity_percent_from_alpha(alpha: u8) -> u32 {
    ((f64::from(alpha) / 255.0) * 100.0).round() as u32
}
