use std::path::PathBuf;

use crate::config::{ConfigFileReloader, GromaqConfig};
use crate::pty::ShellCommand;
use crate::renderer::RendererConfig;

use super::native_input::{NativeResizeGridMapper, clamp_u32_to_u16};
use super::{
    NativeAppConfig, NativeAppError, NativeAppEventProxy, NativeTerminalApp,
    NativeTerminalRuntimeConfig, RealNativePtySpawner,
};

impl NativeTerminalApp {
    /// Install a config-file reloader for live reloadable settings.
    pub fn set_config_reloader(&mut self, config_reloader: ConfigFileReloader) {
        self.config_reloader = Some(config_reloader);
    }

    /// Poll the installed config file and apply reloadable settings when it changed.
    pub fn reload_config_if_changed(&mut self) -> Result<bool, NativeAppError> {
        let Some(reload) = self
            .config_reloader
            .as_mut()
            .map(ConfigFileReloader::reload_if_changed)
            .transpose()
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?
        else {
            return Ok(false);
        };
        if !reload.changed {
            return Ok(false);
        }
        self.apply_reloadable_gromaq_config(&reload.config)?;
        Ok(true)
    }

    /// Apply validated user configuration fields that are reloadable without restarting the PTY.
    pub fn apply_reloadable_gromaq_config(
        &mut self,
        config: &GromaqConfig,
    ) -> Result<(), NativeAppError> {
        let app_config = NativeAppConfig::from_gromaq_config(config)?;
        let reloaded_shell = shell_command_from_gromaq_config(config);
        let shell_changed = self.runtime.config().shell != reloaded_shell;
        let mut runtime_config =
            NativeTerminalRuntimeConfig::from_gromaq_config(config, reloaded_shell.clone())?;
        let (reference_width_px, reference_height_px, pixel_width, pixel_height) =
            self.reload_reference_size(&app_config);
        runtime_config.pixel_width = pixel_width;
        runtime_config.pixel_height = pixel_height;
        let renderer_config = RendererConfig::from_gromaq_config(config)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        let resize_mapper = NativeResizeGridMapper::new(
            renderer_config.cell_width_px,
            renderer_config.line_height_px,
            renderer_config.surface_padding_px,
        )
        .ok_or_else(|| {
            NativeAppError::Runtime("native renderer cell dimensions must be non-zero".to_owned())
        })?;
        if let Some(resize) =
            resize_mapper.resize_for_window(reference_width_px, reference_height_px)
        {
            runtime_config.terminal_cols = resize.cols;
            runtime_config.terminal_rows = resize.rows;
            runtime_config.pixel_width = resize.pixel_width;
            runtime_config.pixel_height = resize.pixel_height;
        }
        let terminal_config_changed = self.runtime.config().terminal_cols
            != runtime_config.terminal_cols
            || self.runtime.config().terminal_rows != runtime_config.terminal_rows
            || self.runtime.config().scrollback_lines != runtime_config.scrollback_lines
            || self.runtime.config().pixel_width != runtime_config.pixel_width
            || self.runtime.config().pixel_height != runtime_config.pixel_height
            || self.runtime.config().cursor_shape != runtime_config.cursor_shape
            || self.runtime.config().cursor_blinking != runtime_config.cursor_blinking;
        if terminal_config_changed {
            self.runtime.reconfigure_terminal(runtime_config)?;
        }
        if shell_changed {
            if self.runtime.has_shell_session() {
                self.runtime
                    .restart_shell(reloaded_shell, &self.pty_spawner)?;
            } else {
                self.runtime.set_shell_command(reloaded_shell);
            }
        }
        self.resize_mapper = resize_mapper;
        self.lifecycle.apply_config(app_config);
        self.renderer.reconfigure(renderer_config);
        self.runtime.invalidate_terminal_frame();
        Ok(())
    }

    fn reload_reference_size(&self, app_config: &NativeAppConfig) -> (u32, u32, u16, u16) {
        if let Some(window) = &self.window {
            let size = window.inner_size();
            if size.width > 0 && size.height > 0 {
                return (
                    size.width,
                    size.height,
                    clamp_u32_to_u16(size.width),
                    clamp_u32_to_u16(size.height),
                );
            }
        }
        (
            app_config.width,
            app_config.height,
            self.runtime.config().pixel_width,
            self.runtime.config().pixel_height,
        )
    }

    /// Configure the user-event proxy used by the PTY background reader.
    pub fn set_event_proxy(&mut self, event_proxy: NativeAppEventProxy) {
        self.pty_spawner = RealNativePtySpawner::with_event_proxy(event_proxy);
    }
}

fn shell_command_from_gromaq_config(config: &GromaqConfig) -> ShellCommand {
    let mut shell = config
        .shell
        .program
        .as_ref()
        .map(|program| ShellCommand {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
        })
        .unwrap_or_else(ShellCommand::default_shell);
    shell.args = config.shell.args.iter().map(Into::into).collect();
    shell.cwd = config.shell.cwd.as_ref().map(PathBuf::from);
    shell
}
