use std::fs;
use std::path::Path;

use crate::font::RasterizedGlyphCache;
use crate::renderer::{
    PreparedSurfaceGlyphFrame, PreparedSurfaceGlyphFrameConfig, SurfaceBackend,
    SurfaceFrameBackend, SurfaceFrameError, WgpuRenderer,
};

use super::snapshot::prepared_frame_ppm_bytes;
use super::{NativeGlyphFrameError, NativeTerminalRuntime};

mod report;
mod tmux_report;
mod window;

pub use report::NativeGlyphFramePresentation;
use tmux_report::{plan_contains_tmux_status_pane_command, plan_has_current_startup_copy};
pub use window::NativeWindowSurface;

/// Render dirty terminal state into a prepared glyph frame and present it through a native surface.
pub fn render_and_present_terminal_glyph_frame<S, B>(
    runtime: &mut NativeTerminalRuntime<S>,
    renderer: &mut WgpuRenderer,
    glyph_cache: &mut RasterizedGlyphCache,
    surface: &mut NativeWindowSurface<B>,
) -> Result<bool, NativeGlyphFrameError>
where
    B: SurfaceBackend + SurfaceFrameBackend,
{
    render_and_present_terminal_glyph_frame_report(runtime, renderer, glyph_cache, surface)
        .map(|report| report.glyph_frame_presented)
}

/// Render dirty terminal state into a prepared glyph frame, present it, and return presentation metrics.
pub fn render_and_present_terminal_glyph_frame_report<S, B>(
    runtime: &mut NativeTerminalRuntime<S>,
    renderer: &mut WgpuRenderer,
    glyph_cache: &mut RasterizedGlyphCache,
    surface: &mut NativeWindowSurface<B>,
) -> Result<NativeGlyphFramePresentation, NativeGlyphFrameError>
where
    B: SurfaceBackend + SurfaceFrameBackend,
{
    render_and_present_terminal_glyph_frame_report_with_snapshot(
        runtime,
        renderer,
        glyph_cache,
        surface,
        None,
    )
}

/// Render, present, and optionally export a PPM preview of the prepared native glyph frame.
pub fn render_and_present_terminal_glyph_frame_report_with_snapshot<S, B>(
    runtime: &mut NativeTerminalRuntime<S>,
    renderer: &mut WgpuRenderer,
    glyph_cache: &mut RasterizedGlyphCache,
    surface: &mut NativeWindowSurface<B>,
    snapshot_path: Option<&Path>,
) -> Result<NativeGlyphFramePresentation, NativeGlyphFrameError>
where
    B: SurfaceBackend + SurfaceFrameBackend,
{
    render_and_present_terminal_glyph_frame_report_with_snapshot_and_status_overlay(
        runtime,
        renderer,
        glyph_cache,
        surface,
        snapshot_path,
        None,
    )
}

pub(super) fn render_and_present_terminal_glyph_frame_report_with_snapshot_and_status_overlay<
    S,
    B,
>(
    runtime: &mut NativeTerminalRuntime<S>,
    renderer: &mut WgpuRenderer,
    glyph_cache: &mut RasterizedGlyphCache,
    surface: &mut NativeWindowSurface<B>,
    snapshot_path: Option<&Path>,
    status_overlay: Option<&str>,
) -> Result<NativeGlyphFramePresentation, NativeGlyphFrameError>
where
    B: SurfaceBackend + SurfaceFrameBackend,
{
    // Swapchain frames are not retained. Until native partial-present support exists,
    // every surface presentation must redraw the full visible terminal contents.
    runtime.invalidate_terminal_frame();
    let clear_color = renderer.config().clear_color;
    if !runtime.render_terminal_frame_with_status_overlay(renderer, status_overlay)? {
        return clear_and_present_report(surface, clear_color);
    }
    let tmux_status_strip_rendered = runtime.last_rendered_tmux_status_strip();
    let tmux_manager_panel_rendered = runtime.last_rendered_tmux_manager_panel();
    let (tmux_manager_sessions, tmux_manager_windows, tmux_manager_panes) =
        runtime.last_rendered_tmux_manager_state_counts();
    let Some(plan) = renderer.last_plan() else {
        let mut report = clear_and_present_report(surface, clear_color)?;
        report.rendered = true;
        report.tmux_status_strip_rendered = tmux_status_strip_rendered;
        report.tmux_manager_panel_rendered = tmux_manager_panel_rendered;
        report.tmux_manager_sessions = tmux_manager_sessions;
        report.tmux_manager_windows = tmux_manager_windows;
        report.tmux_manager_panes = tmux_manager_panes;
        return Ok(report);
    };
    let tmux_status_pane_command_rendered = plan_contains_tmux_status_pane_command(runtime, plan);
    let glyphs = glyph_cache.rasterize_plan(plan)?;
    let prepared = PreparedSurfaceGlyphFrame::from_render_plan(
        plan,
        &glyphs.bitmaps,
        PreparedSurfaceGlyphFrameConfig {
            cell_width_px: renderer.config().cell_width_px,
            line_height_px: renderer.config().line_height_px,
            clear_color,
            cursor_color_rgba8: renderer.config().cursor_color_rgba8,
            surface_padding_px: renderer.config().surface_padding_px,
            cell_spacing_px: renderer.config().cell_spacing_px,
        },
    )?;
    let frame = prepared.as_surface_glyph_frame();
    let snapshot = match snapshot_path {
        Some(path) => {
            let preview = prepared.preview_rgba8()?;
            let bytes = prepared_frame_ppm_bytes(preview.width, preview.height, &preview.rgba)?;
            Some((path, preview.width, preview.height, bytes))
        }
        None => None,
    };
    let mut report = NativeGlyphFramePresentation {
        rendered: true,
        glyph_frame_presented: false,
        tmux_status_strip_rendered,
        tmux_status_pane_command_rendered,
        tmux_manager_panel_rendered,
        default_startup_content_checked: plan_has_current_startup_copy(plan),
        tmux_manager_sessions,
        tmux_manager_windows,
        tmux_manager_panes,
        clear_presented: false,
        width: frame.width,
        height: frame.height,
        glyph_quads: frame.batch.quads.len(),
        background_quads: frame.background_batch.quads.len(),
        decoration_quads: frame.decoration_batch.quads.len(),
        cursor_quads: frame.cursor_batch.quads.len(),
        atlas_bytes: frame.atlas.rgba.len(),
        atlas_occupied_slots: frame.atlas.occupied_slots,
        snapshot_written: false,
        snapshot_bytes: 0,
        snapshot_width: 0,
        snapshot_height: 0,
    };
    if let Some((path, width, height, bytes)) = snapshot {
        create_snapshot_parent_dir(path)?;
        fs::write(path, &bytes).map_err(|error| {
            NativeGlyphFrameError::Snapshot(format!(
                "failed to write native glyph frame snapshot: {error}"
            ))
        })?;
        report.snapshot_written = true;
        report.snapshot_bytes = bytes.len();
        report.snapshot_width = width;
        report.snapshot_height = height;
    }
    match surface.present_glyph_frame(frame) {
        Ok(()) => {
            report.glyph_frame_presented = true;
        }
        Err(SurfaceFrameError::Timeout | SurfaceFrameError::Occluded)
            if report.snapshot_written =>
        {
            return Ok(report);
        }
        Err(error) => return Err(error.into()),
    }
    Ok(report)
}
fn create_snapshot_parent_dir(path: &Path) -> Result<(), NativeGlyphFrameError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent).map_err(|error| {
        NativeGlyphFrameError::Snapshot(format!(
            "failed to create native glyph frame snapshot directory: {error}"
        ))
    })
}

fn clear_and_present_report<B>(
    surface: &mut NativeWindowSurface<B>,
    clear_color: [f64; 4],
) -> Result<NativeGlyphFramePresentation, NativeGlyphFrameError>
where
    B: SurfaceBackend + SurfaceFrameBackend,
{
    let (width, height) = surface.configured_size().unwrap_or_default();
    surface.clear_and_present(clear_color)?;
    Ok(NativeGlyphFramePresentation {
        rendered: false,
        glyph_frame_presented: false,
        clear_presented: true,
        width,
        height,
        ..NativeGlyphFramePresentation::default()
    })
}
