use std::{fs, path::PathBuf};

use super::CliExit;
use crate::app::{NativeAppConfig, NativeTerminalApp, NativeTerminalRuntimeConfig};
use crate::config::ConfigFileReloader;
use crate::renderer::RendererConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeConfigReloadSmokeReport {
    unchanged_poll_changed: bool,
    changed_poll_changed: bool,
    cols: u16,
    rows: u16,
    scrollback_lines: usize,
    target_fps: u32,
    dirty_regions: bool,
    font_size_px: u16,
    cell_width_px: u16,
    line_height_px: u16,
    shell_program: String,
}

pub(super) fn runtime_config_reload_smoke_exit() -> CliExit {
    match run_runtime_config_reload_smoke() {
        Ok(report) => CliExit {
            code: 0,
            stdout: format!(
                "runtime config reload smoke: ok\nunchanged poll changed: {}\nchanged poll changed: {}\nterminal: {}x{}\nscrollback lines: {}\ntarget fps: {}\ndirty-region rendering: {}\nfont size px: {}\ncell width px: {}\nline height px: {}\nshell: {}\n",
                report.unchanged_poll_changed,
                report.changed_poll_changed,
                report.cols,
                report.rows,
                report.scrollback_lines,
                report.target_fps,
                report.dirty_regions,
                report.font_size_px,
                report.cell_width_px,
                report.line_height_px,
                report.shell_program,
            ),
            stderr: String::new(),
        },
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("runtime config reload smoke failed: {error}\n"),
        },
    }
}

fn run_runtime_config_reload_smoke() -> std::result::Result<RuntimeConfigReloadSmokeReport, String>
{
    let path = runtime_config_reload_smoke_path()?;
    let result = run_runtime_config_reload_smoke_with_path(&path);
    let _ = fs::remove_file(path);
    result
}

fn run_runtime_config_reload_smoke_with_path(
    path: &PathBuf,
) -> std::result::Result<RuntimeConfigReloadSmokeReport, String> {
    fs::write(path, "[performance]\ntarget_fps = 144\n")
        .map_err(|error| format!("failed to write initial config: {error}"))?;
    let mut app = NativeTerminalApp::new_with_runtime_and_renderer_config(
        NativeAppConfig::default(),
        NativeTerminalRuntimeConfig::default(),
        RendererConfig::default(),
    )
    .map_err(|error| error.to_string())?;
    let reloader =
        ConfigFileReloader::from_file(path.clone()).map_err(|error| error.to_string())?;
    app.set_config_reloader(reloader);

    let unchanged_poll_changed = app
        .reload_config_if_changed()
        .map_err(|error| error.to_string())?;
    if unchanged_poll_changed {
        return Err("unchanged config poll reported a reload".to_owned());
    }

    fs::write(
        path,
        r#"
        [terminal]
        cols = 28
        rows = 6
        scrollback_lines = 96

        [performance]
        target_fps = 120
        dirty_region_rendering = false

        [font]
        size_px = 18.0

        [shell]
        program = "/bin/sh"
        args = ["-l"]
        cwd = "/tmp"
        "#,
    )
    .map_err(|error| format!("failed to write changed config: {error}"))?;

    let changed_poll_changed = app
        .reload_config_if_changed()
        .map_err(|error| error.to_string())?;
    if !changed_poll_changed {
        return Err("changed config poll did not report a reload".to_owned());
    }

    let grid = app.runtime().terminal().dump_grid();
    let runtime_config = app.runtime().config();
    let app_config = app.lifecycle().config();
    let renderer_config = app.renderer().config();
    if grid.cols != 125 || grid.rows != 23 {
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
        shell_program: runtime_config.shell.program.display().to_string(),
    })
}

fn runtime_config_reload_smoke_path() -> std::result::Result<PathBuf, String> {
    let directory = std::env::current_dir()
        .map_err(|error| format!("failed to resolve current directory: {error}"))?
        .join("target")
        .join("gromaq-runtime-smokes");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("failed to create smoke directory: {error}"))?;
    Ok(directory.join(format!(
        "{}-runtime-config-reload-smoke.toml",
        std::process::id()
    )))
}
