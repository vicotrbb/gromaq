//! Native `winit` application loop boundary.

use std::path::Path;
use std::sync::Arc;

use winit::dpi::PhysicalPosition;
use winit::event_loop::EventLoop;
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

use crate::config::ConfigFileReloader;
use crate::font::RasterizedGlyphCache;
use crate::native_gpu::NativeGpuContext;
use crate::pty::PtySession;
use crate::renderer::{RendererConfig, WgpuRenderer, WgpuSurfaceBackend};

mod config_reload;
mod errors;
mod handler;
mod lifecycle;
mod native_input;
mod perf;
mod presentation;
mod pty_bridge;
mod runtime;
mod surface;
pub use errors::{NativeAppError, NativeGlyphFrameError};
pub use lifecycle::{
    NativeAppAction, NativeAppConfig, NativeAppEvent, NativeAppEventProxy, NativeAppLifecycle,
    NativeAppRunReport,
};
pub use native_input::{
    NativeMouseButtonTracker, NativeMouseGridMapper, NativePtyResize, NativeResizeGridMapper,
    NativeWindowMouseInput, is_native_copy_shortcut, is_native_paste_shortcut,
};
use native_input::{native_mouse_button, wheel_mouse_button};
pub use perf::{NativeRuntimePerfSnapshot, NativeRuntimeStateSnapshot};
pub use pty_bridge::{
    NativePtySessionIo, NativePtySpawner, NativeTerminalRuntimeConfig, RealNativePtySpawner,
};
pub use runtime::NativeTerminalRuntime;
pub use surface::{
    NativeGlyphFramePresentation, NativeWindowSurface, load_default_native_glyph_cache,
    render_and_present_terminal_glyph_frame, render_and_present_terminal_glyph_frame_report,
};

/// Native terminal application handler for the `winit` event loop.
pub struct NativeTerminalApp {
    lifecycle: NativeAppLifecycle,
    runtime: NativeTerminalRuntime<PtySession>,
    renderer: WgpuRenderer,
    glyph_cache: RasterizedGlyphCache,
    pty_spawner: RealNativePtySpawner,
    gpu_context: Option<NativeGpuContext>,
    surface: Option<NativeWindowSurface<WgpuSurfaceBackend<'static>>>,
    modifiers: ModifiersState,
    cursor_position: Option<PhysicalPosition<f64>>,
    mouse_buttons: NativeMouseButtonTracker,
    resize_mapper: NativeResizeGridMapper,
    config_reloader: Option<ConfigFileReloader>,
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    startup_error: Option<String>,
}

impl NativeTerminalApp {
    /// Create a native terminal app handler.
    pub fn new(config: NativeAppConfig) -> Result<Self, NativeAppError> {
        Self::new_with_runtime_config(config, NativeTerminalRuntimeConfig::default())
    }

    /// Create a native terminal app handler with an explicit runtime configuration.
    pub fn new_with_runtime_config(
        config: NativeAppConfig,
        runtime_config: NativeTerminalRuntimeConfig,
    ) -> Result<Self, NativeAppError> {
        Self::new_with_runtime_and_renderer_config(
            config,
            runtime_config,
            RendererConfig::default(),
        )
    }

    /// Create a native terminal app handler with explicit runtime and renderer configuration.
    pub fn new_with_runtime_and_renderer_config(
        config: NativeAppConfig,
        runtime_config: NativeTerminalRuntimeConfig,
        renderer_config: RendererConfig,
    ) -> Result<Self, NativeAppError> {
        let resize_mapper = NativeResizeGridMapper::new(
            config.width,
            config.height,
            runtime_config.terminal_cols,
            runtime_config.terminal_rows,
        )
        .ok_or_else(|| {
            NativeAppError::Runtime(
                "native window and terminal reference dimensions must be non-zero".to_owned(),
            )
        })?;
        let runtime = NativeTerminalRuntime::new(runtime_config)?;
        Ok(Self {
            lifecycle: NativeAppLifecycle::new(config),
            runtime,
            renderer: WgpuRenderer::new(renderer_config)?,
            glyph_cache: load_default_native_glyph_cache()?,
            pty_spawner: RealNativePtySpawner::default(),
            gpu_context: None,
            surface: None,
            modifiers: ModifiersState::empty(),
            cursor_position: None,
            mouse_buttons: NativeMouseButtonTracker::default(),
            resize_mapper,
            config_reloader: None,
            window: None,
            window_id: None,
            startup_error: None,
        })
    }

    /// Access lifecycle state.
    pub fn lifecycle(&self) -> &NativeAppLifecycle {
        &self.lifecycle
    }

    /// Access runtime state.
    pub fn runtime(&self) -> &NativeTerminalRuntime<PtySession> {
        &self.runtime
    }

    /// Access renderer state.
    pub fn renderer(&self) -> &WgpuRenderer {
        &self.renderer
    }

    /// Take a startup error captured from the event handler.
    pub fn take_startup_error(&mut self) -> Option<String> {
        self.startup_error.take()
    }
}

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
    let event_loop = EventLoop::<NativeAppEvent>::with_user_event().build()?;
    let event_proxy = event_loop.create_proxy();
    let mut app = NativeTerminalApp::new_with_runtime_and_renderer_config(
        config,
        runtime_config,
        renderer_config,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_terminal_app_new_rejects_zero_window_reference_size() {
        let config = NativeAppConfig {
            width: 0,
            ..NativeAppConfig::default()
        };
        let error = match NativeTerminalApp::new(config) {
            Ok(_) => panic!("zero-width native app config should be rejected"),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "native runtime failed: native window and terminal reference dimensions must be non-zero"
        );
    }

    #[test]
    fn native_terminal_app_new_loads_default_glyph_cache() {
        let app = NativeTerminalApp::new(NativeAppConfig::default()).unwrap();

        assert!(app.glyph_cache.is_empty());
    }
}
