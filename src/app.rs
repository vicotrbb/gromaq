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
mod text_zoom;
pub use errors::{NativeAppError, NativeGlyphFrameError};
pub use lifecycle::{
    NativeAppAction, NativeAppConfig, NativeAppEvent, NativeAppEventProxy, NativeAppLifecycle,
    NativeAppRunReport,
};
pub use native_input::{
    NativeMouseButtonTracker, NativeMouseGridMapper, NativePtyResize, NativeResizeGridMapper,
    NativeTextZoomAction, NativeWindowMouseInput, is_native_copy_shortcut,
    is_native_paste_shortcut, native_text_zoom_action,
};
use native_input::{native_mouse_button, wheel_mouse_button};
pub use perf::{NativeRuntimePerfSnapshot, NativeRuntimeStateSnapshot};
pub use pty_bridge::{
    NativePtySessionIo, NativePtySpawner, NativeTerminalRuntimeConfig, RealNativePtySpawner,
};
pub use runtime::NativeTerminalRuntime;
pub use surface::{
    NativeGlyphFramePresentation, NativeWindowSurface, load_default_native_glyph_cache,
    load_native_glyph_cache, render_and_present_terminal_glyph_frame,
    render_and_present_terminal_glyph_frame_report,
};
use text_zoom::renderer_config_for_text_zoom;

/// Native terminal application handler for the `winit` event loop.
pub struct NativeTerminalApp {
    lifecycle: NativeAppLifecycle,
    runtime: NativeTerminalRuntime<PtySession>,
    renderer: WgpuRenderer,
    glyph_cache: RasterizedGlyphCache,
    font_family: String,
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
        Self::new_with_runtime_renderer_and_font_config(
            config,
            runtime_config,
            renderer_config,
            "monospace",
        )
    }

    /// Create a native terminal app with explicit runtime, renderer, and font configuration.
    pub fn new_with_runtime_renderer_and_font_config(
        config: NativeAppConfig,
        mut runtime_config: NativeTerminalRuntimeConfig,
        renderer_config: RendererConfig,
        font_family: impl Into<String>,
    ) -> Result<Self, NativeAppError> {
        let font_family = font_family.into();
        if config.width == 0 || config.height == 0 {
            return Err(NativeAppError::Runtime(
                "native window dimensions must be non-zero".to_owned(),
            ));
        }
        let resize_mapper = NativeResizeGridMapper::new(
            renderer_config.cell_width_px,
            renderer_config.line_height_px,
            renderer_config.surface_padding_px,
        )
        .ok_or_else(|| {
            NativeAppError::Runtime("native renderer cell dimensions must be non-zero".to_owned())
        })?;
        if let Some(resize) = resize_mapper.resize_for_window(config.width, config.height) {
            runtime_config.terminal_cols = resize.cols;
            runtime_config.terminal_rows = resize.rows;
            runtime_config.pixel_width = resize.pixel_width;
            runtime_config.pixel_height = resize.pixel_height;
        }
        let runtime = NativeTerminalRuntime::new(runtime_config)?;
        Ok(Self {
            lifecycle: NativeAppLifecycle::new(config),
            runtime,
            renderer: WgpuRenderer::new(renderer_config)?,
            glyph_cache: load_native_glyph_cache(&font_family)?,
            font_family,
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

    /// Active configured font family or file path used by the native glyph cache.
    pub fn font_family(&self) -> &str {
        &self.font_family
    }

    /// Apply a browser-style terminal text zoom action to the active renderer metrics.
    pub fn apply_text_zoom_action(
        &mut self,
        action: NativeTextZoomAction,
    ) -> Result<bool, NativeAppError> {
        let current = self.renderer.config().clone();
        let next = renderer_config_for_text_zoom(&current, action);
        if next == current {
            return Ok(false);
        }
        self.apply_renderer_config_to_current_viewport(next)?;
        Ok(true)
    }

    /// Take a startup error captured from the event handler.
    pub fn take_startup_error(&mut self) -> Option<String> {
        self.startup_error.take()
    }

    fn apply_renderer_config_to_current_viewport(
        &mut self,
        renderer_config: RendererConfig,
    ) -> Result<(), NativeAppError> {
        let resize_mapper = NativeResizeGridMapper::new(
            renderer_config.cell_width_px,
            renderer_config.line_height_px,
            renderer_config.surface_padding_px,
        )
        .ok_or_else(|| {
            NativeAppError::Runtime("native renderer cell dimensions must be non-zero".to_owned())
        })?;
        let (width, height) = self
            .window
            .as_ref()
            .map(|window| {
                let size = window.inner_size();
                (size.width, size.height)
            })
            .unwrap_or_else(|| {
                (
                    self.lifecycle.config().width,
                    self.lifecycle.config().height,
                )
            });
        if let Some(resize) = resize_mapper.resize_for_window(width, height) {
            self.runtime.resize_terminal(resize)?;
        }
        self.resize_mapper = resize_mapper;
        self.renderer.reconfigure(renderer_config);
        self.runtime.invalidate_terminal_frame();
        Ok(())
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
    run_native_app_with_runtime_renderer_font_and_config_file(
        config,
        runtime_config,
        renderer_config,
        "monospace",
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
            "native runtime failed: native window dimensions must be non-zero"
        );
    }

    #[test]
    fn native_terminal_app_new_loads_default_glyph_cache() {
        let app = NativeTerminalApp::new(NativeAppConfig::default()).unwrap();

        assert!(app.glyph_cache.is_empty());
    }
}
