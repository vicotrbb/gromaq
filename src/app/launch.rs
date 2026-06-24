//! Public native app launch wrappers around the `winit` event loop.

use std::path::Path;

use winit::event_loop::EventLoop;

use crate::config::{ConfigFileReloader, DEFAULT_FONT_FAMILY};
use crate::renderer::RendererConfig;

use super::{
    NativeAppConfig, NativeAppError, NativeAppEvent, NativeAppEventProxy, NativeAppRunReport,
    NativeTerminalApp, NativeTerminalRuntimeConfig,
};

/// Run the native `winit` terminal application loop.
pub fn run_native_app(config: NativeAppConfig) -> Result<NativeAppRunReport, NativeAppError> {
    run_native_app_with_runtime_config(config, NativeTerminalRuntimeConfig::default())
}

/// Run the native `winit` terminal application loop with explicit runtime configuration.
pub fn run_native_app_with_runtime_config(
    config: NativeAppConfig,
    runtime_config: NativeTerminalRuntimeConfig,
) -> Result<NativeAppRunReport, NativeAppError> {
    run_native_app_with_runtime_and_renderer_config(
        config,
        runtime_config,
        RendererConfig::default(),
    )
}

/// Run the native `winit` terminal application loop with explicit runtime and renderer config.
pub fn run_native_app_with_runtime_and_renderer_config(
    config: NativeAppConfig,
    runtime_config: NativeTerminalRuntimeConfig,
    renderer_config: RendererConfig,
) -> Result<NativeAppRunReport, NativeAppError> {
    run_native_app_with_runtime_renderer_and_config_file(
        config,
        runtime_config,
        renderer_config,
        None,
    )
}

/// Run the native `winit` terminal application loop with explicit runtime, renderer, and config reload path.
pub fn run_native_app_with_runtime_renderer_and_config_file(
    config: NativeAppConfig,
    runtime_config: NativeTerminalRuntimeConfig,
    renderer_config: RendererConfig,
    config_path: Option<&Path>,
) -> Result<NativeAppRunReport, NativeAppError> {
    run_native_app_with_runtime_renderer_font_and_config_file(
        config,
        runtime_config,
        renderer_config,
        DEFAULT_FONT_FAMILY,
        config_path,
    )
}

/// Run the native `winit` terminal application loop with explicit runtime, renderer, font, and config reload path.
pub fn run_native_app_with_runtime_renderer_font_and_config_file(
    config: NativeAppConfig,
    runtime_config: NativeTerminalRuntimeConfig,
    renderer_config: RendererConfig,
    font_family: impl Into<String>,
    config_path: Option<&Path>,
) -> Result<NativeAppRunReport, NativeAppError> {
    let event_loop = EventLoop::<NativeAppEvent>::with_user_event().build()?;
    let event_proxy = event_loop.create_proxy();
    let mut app = NativeTerminalApp::new_with_runtime_renderer_and_font_config(
        config,
        runtime_config,
        renderer_config,
        font_family,
    )?;
    if let Some(config_path) = config_path {
        app.set_config_reloader(
            ConfigFileReloader::from_file(config_path)
                .map_err(|error| NativeAppError::Runtime(error.to_string()))?,
        );
    }
    app.set_event_proxy(NativeAppEventProxy::from(event_proxy));
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.take_startup_error() {
        return Err(NativeAppError::WindowCreation(error));
    }
    Ok(app.lifecycle().run_report())
}
