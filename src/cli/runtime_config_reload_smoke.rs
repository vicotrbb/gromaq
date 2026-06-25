use std::{fs, path::PathBuf};

use crate::app::{NativeAppConfig, NativeTerminalApp, NativeTerminalRuntimeConfig};
use crate::config::ConfigFileReloader;
use crate::renderer::RendererConfig;

mod output;
mod validation;

use super::CliExit;
use output::{runtime_config_reload_smoke_error, runtime_config_reload_smoke_success};
use validation::validate_runtime_config_reload_smoke;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeConfigReloadSmokeReport {
    pub(super) unchanged_poll_changed: bool,
    pub(super) changed_poll_changed: bool,
    pub(super) cols: u16,
    pub(super) rows: u16,
    pub(super) scrollback_lines: usize,
    pub(super) target_fps: u32,
    pub(super) dirty_regions: bool,
    pub(super) font_size_px: u16,
    pub(super) cell_width_px: u16,
    pub(super) line_height_px: u16,
    pub(super) cell_spacing_px: u16,
    pub(super) shell_program: String,
}

pub(super) fn runtime_config_reload_smoke_exit() -> CliExit {
    match run_runtime_config_reload_smoke() {
        Ok(report) => runtime_config_reload_smoke_success(&report),
        Err(error) => runtime_config_reload_smoke_error(error),
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
        line_height_px = 22.0

        [theme]
        cell_spacing_px = 2

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

    validate_runtime_config_reload_smoke(&app, unchanged_poll_changed, changed_poll_changed)
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
