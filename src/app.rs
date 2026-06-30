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
mod icon;
mod launch;
mod lifecycle;
mod native_input;
mod perf;
mod presentation;
mod pty_bridge;
mod runtime;
mod snapshot;
mod surface;
#[cfg(test)]
mod tests;
mod text_zoom;
mod text_zoom_action;
mod viewport_resize;
mod welcome;
pub use errors::{NativeAppError, NativeGlyphFrameError};
pub use fonts::{
    NativeFontResolution, load_default_native_glyph_cache, load_native_glyph_cache,
    load_native_glyph_cache_with_fallbacks, resolve_native_font_paths,
    resolve_native_font_paths_with_fallbacks,
};
pub use launch::{
    run_native_app, run_native_app_with_runtime_and_renderer_config,
    run_native_app_with_runtime_config, run_native_app_with_runtime_renderer_and_config_file,
    run_native_app_with_runtime_renderer_font_and_config_file,
    run_native_app_with_runtime_renderer_font_fallbacks_and_config_file,
};
pub use lifecycle::{
    NativeAppAction, NativeAppConfig, NativeAppEvent, NativeAppEventProxy, NativeAppLifecycle,
    NativeAppRunReport,
};
pub use native_input::{
    NativeMouseButtonTracker, NativeMouseGridMapper, NativePtyResize, NativeRenderedGridMetrics,
    NativeResizeGridMapper, NativeTextZoomAction, NativeTmuxAssistAction, NativeWindowMouseInput,
    NativeWindowMouseInputResult, is_native_copy_shortcut, is_native_paste_shortcut,
    native_text_zoom_action, native_tmux_assist_action, native_wheel_text_zoom_action,
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
pub(crate) use welcome::{WELCOME_AVATAR_ANSI, default_welcome_text};

/// Native terminal application handler for the `winit` event loop.
pub struct NativeTerminalApp {
    lifecycle: NativeAppLifecycle,
    runtime: NativeTerminalRuntime<PtySession>,
    renderer: WgpuRenderer,
    glyph_cache: RasterizedGlyphCache,
    font_family: String,
    font_fallback_families: Vec<String>,
    pty_spawner: RealNativePtySpawner,
    gpu_context: Option<NativeGpuContext>,
    surface: Option<NativeWindowSurface<WgpuSurfaceBackend<'static>>>,
    modifiers: ModifiersState,
    ime_preedit_active: bool,
    cursor_position: Option<PhysicalPosition<f64>>,
    mouse_buttons: NativeMouseButtonTracker,
    resize_mapper: NativeResizeGridMapper,
    config_reloader: Option<ConfigFileReloader>,
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    startup_error: Option<String>,
}
