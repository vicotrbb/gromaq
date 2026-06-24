use std::path::PathBuf;

use thiserror::Error;

use super::CliExit;
use crate::app::{
    NativeAppConfig, NativeTerminalRuntimeConfig,
    run_native_app_with_runtime_renderer_and_config_file,
};
use crate::config::{GromaqConfig, ShellSettings};
use crate::pty::ShellCommand;
use crate::renderer::RendererConfig;

/// Error returned by the native app launcher boundary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("native app launch failed: {message}")]
pub struct NativeAppLaunchError {
    message: String,
}

impl NativeAppLaunchError {
    /// Create a native app launch error from a displayable message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Native launch configuration derived from defaults or a user config file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NativeAppLaunchConfig {
    /// Window and frame-pacing configuration.
    pub app: NativeAppConfig,
    /// Terminal, scrollback, and shell runtime configuration.
    pub runtime: NativeTerminalRuntimeConfig,
    /// Renderer configuration for glyph planning and frame presentation.
    pub renderer: RendererConfig,
    /// Optional TOML config path to poll for reloadable changes after launch.
    pub config_path: Option<PathBuf>,
}

impl NativeAppLaunchConfig {
    /// Build a launch configuration from a validated user configuration.
    pub fn from_gromaq_config(config: &GromaqConfig) -> Result<Self, NativeAppLaunchError> {
        let app = NativeAppConfig::from_gromaq_config(config)
            .map_err(|error| NativeAppLaunchError::new(error.to_string()))?;
        let shell = shell_command_from_settings(&config.shell);
        let runtime = NativeTerminalRuntimeConfig::from_gromaq_config(config, shell)
            .map_err(|error| NativeAppLaunchError::new(error.to_string()))?;
        let renderer = RendererConfig::from_gromaq_config(config)
            .map_err(|error| NativeAppLaunchError::new(error.to_string()))?;
        Ok(Self {
            app,
            runtime,
            renderer,
            config_path: None,
        })
    }
}

/// Launches the native terminal app for the no-argument CLI path.
pub trait NativeAppLauncher {
    /// Launch the native app using `config`.
    fn launch(&self, config: NativeAppLaunchConfig) -> Result<(), NativeAppLaunchError>;
}

/// Production native app launcher.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RealNativeAppLauncher;

impl NativeAppLauncher for RealNativeAppLauncher {
    fn launch(&self, config: NativeAppLaunchConfig) -> Result<(), NativeAppLaunchError> {
        run_native_app_with_runtime_renderer_and_config_file(
            config.app,
            config.runtime,
            config.renderer,
            config.config_path.as_deref(),
        )
        .map_err(|error| NativeAppLaunchError::new(error.to_string()))
    }
}

pub(super) fn launch_config_file_exit<A>(path: &str, app_launcher: &A) -> CliExit
where
    A: NativeAppLauncher,
{
    let config = match GromaqConfig::from_toml_file(path) {
        Ok(config) => config,
        Err(error) => {
            return CliExit {
                code: 1,
                stdout: String::new(),
                stderr: format!("config launch failed: {error}\n"),
            };
        }
    };
    let launch_config = match NativeAppLaunchConfig::from_gromaq_config(&config) {
        Ok(mut launch_config) => {
            launch_config.config_path = Some(PathBuf::from(path));
            launch_config
        }
        Err(error) => {
            return CliExit {
                code: 1,
                stdout: String::new(),
                stderr: format!("{error}\n"),
            };
        }
    };
    launch_native_app_exit(app_launcher, launch_config)
}

pub(super) fn launch_native_app_exit<A>(app_launcher: &A, config: NativeAppLaunchConfig) -> CliExit
where
    A: NativeAppLauncher,
{
    match app_launcher.launch(config) {
        Ok(()) => CliExit {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        },
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("{error}\n"),
        },
    }
}

pub(super) fn config_check_exit(path: &str) -> CliExit {
    match GromaqConfig::from_toml_file(path) {
        Ok(config) => CliExit {
            code: 0,
            stdout: format!(
                "config check: ok\npath: {}\nterminal: {}x{}\nscrollback lines: {}\nshell: {}\nshell args: {}\nshell cwd: {}\nfont: {} {}px\ntarget fps: {}\ndirty-region rendering: {}\n",
                path,
                config.terminal.cols,
                config.terminal.rows,
                config.terminal.scrollback_lines,
                config.shell.program.as_deref().unwrap_or("<default>"),
                format_config_list(&config.shell.args),
                config.shell.cwd.as_deref().unwrap_or("<default>"),
                config.font.family,
                config.font.size_px,
                config.performance.target_fps,
                config.performance.dirty_region_rendering
            ),
            stderr: String::new(),
        },
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("config check failed: {error}\n"),
        },
    }
}

pub(super) fn config_template_exit() -> CliExit {
    let config = GromaqConfig::default();
    CliExit {
        code: 0,
        stdout: format!(
            "# Gromaq configuration template\n\n[terminal]\ncols = {}\nrows = {}\nscrollback_lines = {}\n\n[shell]\n# program = \"/bin/zsh\"\n# args = [\"-l\"]\n# cwd = \"/tmp\"\n\n[font]\nfamily = \"{}\"\nsize_px = {}\n\n[performance]\ntarget_fps = {}\ndirty_region_rendering = {}\n",
            config.terminal.cols,
            config.terminal.rows,
            config.terminal.scrollback_lines,
            config.font.family,
            config.font.size_px,
            config.performance.target_fps,
            config.performance.dirty_region_rendering
        ),
        stderr: String::new(),
    }
}

fn shell_command_from_settings(settings: &ShellSettings) -> ShellCommand {
    let mut shell = settings
        .program
        .as_ref()
        .map(|program| ShellCommand {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
        })
        .unwrap_or_else(ShellCommand::default_shell);
    shell.args = settings.args.iter().map(Into::into).collect();
    shell.cwd = settings.cwd.as_ref().map(PathBuf::from);
    shell
}

fn format_config_list(values: &[String]) -> String {
    if values.is_empty() {
        "<none>".to_owned()
    } else {
        values.join(" ")
    }
}
