use std::fs;
use std::path::Path;

use crate::app::{
    NativeAppConfig, NativeTerminalRuntimeConfig, default_welcome_text,
    load_default_native_glyph_cache,
};
use crate::config::{GromaqConfig, format_theme_preset};
use crate::renderer::{
    GlyphAtlas, GlyphAtlasConfig, PreparedSurfaceGlyphFrame, PreparedSurfaceGlyphFrameConfig,
    RenderPlanner, RendererConfig,
};
use crate::{Terminal, TerminalConfig};

use super::super::CliExit;
use super::ppm::ppm_bytes;

const WELCOME_PREVIEW_COLS: u16 = 80;
const WELCOME_PREVIEW_ROWS: u16 = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WelcomePreviewReport {
    bytes_written: usize,
    width: u32,
    height: u32,
    glyph_quads: usize,
    cursor_quads: usize,
    atlas_bytes: usize,
}

pub(in crate::cli) fn welcome_preview_snapshot_exit(path: &str) -> CliExit {
    match welcome_preview_snapshot_report(path) {
        Ok(report) => welcome_preview_snapshot_success(path, &report),
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("welcome preview snapshot failed: {error}\n"),
        },
    }
}

fn welcome_preview_snapshot_report(path: &str) -> Result<WelcomePreviewReport, String> {
    let config = preview_config();
    let renderer_config = RendererConfig::from_gromaq_config(&config)
        .map_err(|error| format!("failed to build renderer config: {error}"))?;
    let runtime_config = NativeTerminalRuntimeConfig {
        terminal_cols: WELCOME_PREVIEW_COLS,
        terminal_rows: WELCOME_PREVIEW_ROWS,
        ..NativeTerminalRuntimeConfig::default()
    };
    let mut terminal = Terminal::new(
        TerminalConfig::new(WELCOME_PREVIEW_COLS, WELCOME_PREVIEW_ROWS)
            .map_err(|error| error.to_string())?,
    );
    let text = default_welcome_text(
        &NativeAppConfig::default(),
        &runtime_config,
        &renderer_config,
        &config.font.family,
    );
    terminal
        .write_str(&text)
        .map_err(|error| error.to_string())?;
    terminal
        .write_str("\x1b[?25l")
        .map_err(|error| error.to_string())?;
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
    if plan.glyphs.len() < 120 {
        return Err(format!(
            "welcome preview rendered too few glyphs: {}",
            plan.glyphs.len()
        ));
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
    let snapshot = ppm_bytes(preview.width, preview.height, &preview.rgba)?;
    fs::write(Path::new(path), &snapshot)
        .map_err(|error| format!("failed to write welcome preview snapshot: {error}"))?;
    let frame = prepared.as_surface_glyph_frame();
    Ok(WelcomePreviewReport {
        bytes_written: snapshot.len(),
        width: preview.width,
        height: preview.height,
        glyph_quads: frame.batch.quads.len(),
        cursor_quads: frame.cursor_batch.quads.len(),
        atlas_bytes: frame.atlas.rgba.len(),
    })
}

fn preview_config() -> GromaqConfig {
    GromaqConfig::default()
}

fn welcome_preview_snapshot_success(path: &str, report: &WelcomePreviewReport) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "welcome preview snapshot: ok\npath: {path}\npreset: {}\nbytes written: {}\nframe size: {}x{}\nterminal cells: {}x{}\nglyph quads: {}\ncursor quads: {}\natlas bytes: {}\n",
            format_theme_preset(GromaqConfig::default().theme.preset),
            report.bytes_written,
            report.width,
            report.height,
            WELCOME_PREVIEW_COLS,
            WELCOME_PREVIEW_ROWS,
            report.glyph_quads,
            report.cursor_quads,
            report.atlas_bytes
        ),
        stderr: String::new(),
    }
}
