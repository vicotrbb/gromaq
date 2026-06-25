use std::fs;
use std::path::Path;

use crate::app::load_default_native_glyph_cache;
use crate::renderer::{
    GlyphAtlas, GlyphAtlasConfig, PreparedSurfaceGlyphFrame, PreparedSurfaceGlyphFrameConfig,
    RenderPlanner, RendererConfig,
};
use crate::selection::SelectionRange;
use crate::{Terminal, TerminalConfig};

use super::super::CliExit;

mod pixels;

use pixels::validate_theme_preview_pixels;

const THEME_PREVIEW_COLS: u16 = 56;
const THEME_PREVIEW_ROWS: u16 = 6;
const THEME_PREVIEW_TEXT: &str = "\
\x1b[1mGromaq\x1b[0m native terminal preview\r\n\
\x1b[31merror\x1b[0m  \x1b[32mok\x1b[0m  \x1b[33mwarn\x1b[0m  \x1b[34mblue\x1b[0m  \x1b[35mmagenta\x1b[0m\r\n\
~/Daedalus/gromaq > cargo test --all\r\n\
output stays readable after prompt repaint\r\n";

pub(in crate::cli) fn theme_preview_snapshot_exit(path: &str) -> CliExit {
    match theme_preview_snapshot_report(path) {
        Ok(report) => CliExit {
            code: 0,
            stdout: format!(
                "theme preview snapshot: ok\npath: {path}\nbytes written: {}\nframe size: {}x{}\npreview pixels: {}\nfont size px: {}\ncell width px: {}\nline height px: {}\nsurface padding px: {}\ncell spacing px: {}\nhigh contrast text pixels: {}\nselection pixels: {}\ncursor pixels: {}\nprepared quads: {}\nbackground quads: {}\ncursor quads: {}\natlas bytes: {}\n",
                report.bytes_written,
                report.width,
                report.height,
                report.preview_pixels,
                report.font_size_px,
                report.cell_width_px,
                report.line_height_px,
                report.surface_padding_px,
                report.cell_spacing_px,
                report.high_contrast_text_pixels,
                report.selection_pixels,
                report.cursor_pixels,
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
    cell_spacing_px: u16,
    high_contrast_text_pixels: usize,
    selection_pixels: usize,
    cursor_pixels: usize,
    prepared_quads: usize,
    background_quads: usize,
    cursor_quads: usize,
    atlas_bytes: usize,
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
