use crate::app::NativeTerminalApp;

use super::RuntimeConfigReloadSmokeReport;

pub(super) fn validate_runtime_config_reload_smoke(
    app: &NativeTerminalApp,
    unchanged_poll_changed: bool,
    changed_poll_changed: bool,
) -> std::result::Result<RuntimeConfigReloadSmokeReport, String> {
    let grid = app.runtime().terminal().dump_grid();
    let runtime_config = app.runtime().config();
    let app_config = app.lifecycle().config();
    let renderer_config = app.renderer().config();
    if grid.cols != 104 || grid.rows != 32 {
        return Err(format!(
            "terminal dimensions did not fit reloaded renderer metrics, got {}x{}",
            grid.cols, grid.rows
        ));
    }
    if runtime_config.scrollback_lines != 96 {
        return Err(format!(
            "scrollback lines did not reload, got {}",
            runtime_config.scrollback_lines
        ));
    }
    if app_config.target_fps != 120 || renderer_config.target_fps != 120 {
        return Err(format!(
            "target fps did not reload, app={}, renderer={}",
            app_config.target_fps, renderer_config.target_fps
        ));
    }
    if renderer_config.dirty_regions {
        return Err("dirty-region renderer setting did not reload".to_owned());
    }
    if renderer_config.font_size_px != 18 {
        return Err(format!(
            "renderer font size did not reload, got {}",
            renderer_config.font_size_px
        ));
    }
    if renderer_config.cell_width_px != 10 {
        return Err(format!(
            "renderer cell width did not reload, got {}",
            renderer_config.cell_width_px
        ));
    }
    if renderer_config.line_height_px != 22 {
        return Err(format!(
            "renderer line height did not reload, got {}",
            renderer_config.line_height_px
        ));
    }
    if renderer_config.cell_spacing_px != 2 {
        return Err(format!(
            "renderer cell spacing did not reload, got {}",
            renderer_config.cell_spacing_px
        ));
    }
    if runtime_config.shell.program != "/bin/sh" {
        return Err(format!(
            "shell program did not reload, got {}",
            runtime_config.shell.program.display()
        ));
    }

    Ok(RuntimeConfigReloadSmokeReport {
        unchanged_poll_changed,
        changed_poll_changed,
        cols: grid.cols,
        rows: grid.rows,
        scrollback_lines: runtime_config.scrollback_lines,
        target_fps: app_config.target_fps,
        dirty_regions: renderer_config.dirty_regions,
        font_size_px: renderer_config.font_size_px,
        cell_width_px: renderer_config.cell_width_px,
        line_height_px: renderer_config.line_height_px,
        cell_spacing_px: renderer_config.cell_spacing_px,
        shell_program: runtime_config.shell.program.display().to_string(),
    })
}
