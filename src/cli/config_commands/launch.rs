//! Native app launch configuration and launcher boundary for CLI commands.

use std::path::PathBuf;

use thiserror::Error;

use crate::app::{
    NativeAppConfig, NativeAppRunReport, NativeTerminalRuntimeConfig,
    run_native_app_with_runtime_renderer_font_and_config_file,
};
use crate::config::{DEFAULT_FONT_FAMILY, GromaqConfig, ShellSettings};
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
#[derive(Debug, Clone, PartialEq)]
pub struct NativeAppLaunchConfig {
    /// Window and frame-pacing configuration.
    pub app: NativeAppConfig,
    /// Terminal, scrollback, and shell runtime configuration.
    pub runtime: NativeTerminalRuntimeConfig,
    /// Renderer configuration for glyph planning and frame presentation.
    pub renderer: RendererConfig,
    /// Font family name or explicit font file path for native glyph rasterization.
    pub font_family: String,
    /// Optional TOML config path to poll for reloadable changes after launch.
    pub config_path: Option<PathBuf>,
}

impl Default for NativeAppLaunchConfig {
    fn default() -> Self {
        Self {
            app: NativeAppConfig::default(),
            runtime: NativeTerminalRuntimeConfig::default(),
            renderer: RendererConfig::default(),
            font_family: DEFAULT_FONT_FAMILY.to_owned(),
            config_path: None,
        }
    }
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
            font_family: config.font.family.clone(),
            config_path: None,
        })
    }
}

/// Launches the native terminal app for the no-argument CLI path.
pub trait NativeAppLauncher {
    /// Launch the native app using `config`.
    fn launch(
        &self,
        config: NativeAppLaunchConfig,
    ) -> Result<NativeAppRunReport, NativeAppLaunchError>;
}

/// Production native app launcher.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RealNativeAppLauncher;

impl NativeAppLauncher for RealNativeAppLauncher {
    fn launch(
        &self,
        config: NativeAppLaunchConfig,
    ) -> Result<NativeAppRunReport, NativeAppLaunchError> {
        run_native_app_with_runtime_renderer_font_and_config_file(
            config.app,
            config.runtime,
            config.renderer,
            config.font_family,
            config.config_path.as_deref(),
        )
        .map_err(|error| NativeAppLaunchError::new(error.to_string()))
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
