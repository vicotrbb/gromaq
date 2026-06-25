//! Native `winit` application loop boundary.

use std::sync::Arc;

use winit::dpi::PhysicalPosition;
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

use crate::config::ConfigFileReloader;
use crate::font::RasterizedGlyphCache;
use crate::native_gpu::NativeGpuContext;
use crate::pty::PtySession;
use crate::renderer::{WgpuRenderer, WgpuSurfaceBackend};

mod accessors;
mod config_reload;
mod construction;
mod errors;
mod fonts;
mod handler;
mod launch;
mod lifecycle;
mod native_input;
mod perf;
mod presentation;
mod pty_bridge;
mod runtime;
mod snapshot;
mod surface;
mod text_zoom;
mod text_zoom_action;
mod viewport_resize;
pub use errors::{NativeAppError, NativeGlyphFrameError};
pub use fonts::{
    NativeFontResolution, load_default_native_glyph_cache, load_native_glyph_cache,
    resolve_native_font_paths,
};
pub use launch::{
    run_native_app, run_native_app_with_runtime_and_renderer_config,
    run_native_app_with_runtime_config, run_native_app_with_runtime_renderer_and_config_file,
    run_native_app_with_runtime_renderer_font_and_config_file,
};
pub use lifecycle::{
    NativeAppAction, NativeAppConfig, NativeAppEvent, NativeAppEventProxy, NativeAppLifecycle,
    NativeAppRunReport,
};
pub use native_input::{
    NativeMouseButtonTracker, NativeMouseGridMapper, NativePtyResize, NativeResizeGridMapper,
    NativeTextZoomAction, NativeWindowMouseInput, is_native_copy_shortcut,
    is_native_paste_shortcut, native_text_zoom_action, native_wheel_text_zoom_action,
};
use native_input::{native_mouse_button, wheel_mouse_button};
pub use perf::{NativeRuntimePerfSnapshot, NativeRuntimeStateSnapshot};
pub use pty_bridge::{
    NativePtySessionIo, NativePtySpawner, NativeTerminalRuntimeConfig, RealNativePtySpawner,
};
pub use runtime::NativeTerminalRuntime;
pub use surface::{
    NativeGlyphFramePresentation, NativeWindowSurface, render_and_present_terminal_glyph_frame,
    render_and_present_terminal_glyph_frame_report,
    render_and_present_terminal_glyph_frame_report_with_snapshot,
};

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

    #[test]
    fn native_terminal_app_can_sync_runtime_to_actual_window_pixels() {
        let mut app = NativeTerminalApp::new(NativeAppConfig::default()).unwrap();
        let expected_resize = app.resize_mapper.resize_for_window(2560, 1600).unwrap();

        app.resize_runtime_to_window_pixels(2560, 1600).unwrap();

        assert_eq!(app.runtime.config().pixel_width, 2560);
        assert_eq!(app.runtime.config().pixel_height, 1600);
        assert_eq!(
            app.runtime.terminal().dump_grid().cols,
            expected_resize.cols
        );
        assert_eq!(
            app.runtime.terminal().dump_grid().rows,
            expected_resize.rows
        );
    }
}
