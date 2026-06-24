//! Native `winit` application loop boundary.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use thiserror::Error;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::error::{EventLoopError, OsError};
use winit::event::{ElementState, Ime, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

use crate::clipboard::NativeClipboard;
use crate::config::{ConfigFileReloader, GromaqConfig};
use crate::error::GromaqError;
use crate::font::{FontRasterError, RasterizedGlyphCache};
use crate::mouse::{MouseButton, MouseEventKind};
use crate::native_gpu::{
    GpuBootstrap, GpuBootstrapConfig, GpuBootstrapError, GpuSurfaceError, NativeGpuContext,
};
use crate::pty::{PtySession, ShellCommand};
use crate::renderer::{
    RendererConfig, SurfaceConfigError, SurfaceFrameError, WgpuRenderer, WgpuSurfaceBackend,
};

mod lifecycle;
mod native_input;
mod perf;
mod pty_bridge;
mod runtime;
mod surface;
pub use lifecycle::{
    NativeAppAction, NativeAppConfig, NativeAppEvent, NativeAppEventProxy, NativeAppLifecycle,
};
pub use native_input::{
    NativeMouseButtonTracker, NativeMouseGridMapper, NativePtyResize, NativeResizeGridMapper,
    NativeWindowMouseInput, is_native_copy_shortcut, is_native_paste_shortcut,
};
use native_input::{clamp_u32_to_u16, native_mouse_button, wheel_mouse_button};
pub use perf::{NativeRuntimePerfSnapshot, NativeRuntimeStateSnapshot};
pub use pty_bridge::{
    NativePtySessionIo, NativePtySpawner, NativeTerminalRuntimeConfig, RealNativePtySpawner,
};
pub use runtime::NativeTerminalRuntime;
pub use surface::{
    NativeWindowSurface, load_default_native_glyph_cache, render_and_present_terminal_glyph_frame,
};

/// Native terminal application handler for the `winit` event loop.
pub struct NativeTerminalApp {
    lifecycle: NativeAppLifecycle,
    runtime: NativeTerminalRuntime<PtySession>,
    renderer: WgpuRenderer,
    glyph_cache: Option<RasterizedGlyphCache>,
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
            glyph_cache: load_default_native_glyph_cache().ok(),
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
        let resize_mapper = NativeResizeGridMapper::new(
            reference_width_px,
            reference_height_px,
            runtime_config.terminal_cols,
            runtime_config.terminal_rows,
        )
        .ok_or_else(|| {
            NativeAppError::Runtime(
                "native window and terminal reference dimensions must be non-zero".to_owned(),
            )
        })?;
        let renderer_config = RendererConfig::from_gromaq_config(config)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        let clear_color = self.renderer.config().clear_color;
        let terminal_config_changed = self.runtime.config().terminal_cols
            != runtime_config.terminal_cols
            || self.runtime.config().terminal_rows != runtime_config.terminal_rows
            || self.runtime.config().scrollback_lines != runtime_config.scrollback_lines
            || self.runtime.config().pixel_width != runtime_config.pixel_width
            || self.runtime.config().pixel_height != runtime_config.pixel_height;
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
        self.renderer.reconfigure(RendererConfig {
            clear_color,
            ..renderer_config
        });
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

    /// Take a startup error captured from the event handler.
    pub fn take_startup_error(&mut self) -> Option<String> {
        self.startup_error.take()
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

impl ApplicationHandler<NativeAppEvent> for NativeTerminalApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.lifecycle.on_resumed() != NativeAppAction::CreateWindow {
            return;
        }
        match event_loop.create_window(self.lifecycle.config().window_attributes()) {
            Ok(window) => {
                let window = Arc::new(window);
                let size = window.inner_size();
                self.window_id = Some(window.id());
                if let Err(error) =
                    self.create_surface_for_window(Arc::clone(&window), size.width, size.height)
                {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                    return;
                }
                self.window = Some(window);
                self.lifecycle.on_window_created();
                if let Err(error) = self.runtime.start_shell(&self.pty_spawner) {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            Err(error) => {
                self.startup_error = Some(error.to_string());
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self.reload_config_if_changed() {
            Ok(true) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            Ok(false) => {}
            Err(error) => {
                self.startup_error = Some(error.to_string());
                event_loop.exit();
                return;
            }
        }
        if let Some(deadline) = self.lifecycle.next_pty_pump_deadline(Instant::now()) {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
        let mut clipboard = NativeClipboard::new();
        let action = self
            .runtime
            .pump_output_sync_clipboard_and_schedule_redraw(&mut self.lifecycle, &mut clipboard)
            .unwrap_or_else(|error| {
                self.startup_error = Some(error.to_string());
                NativeAppAction::Exit
            });
        match action {
            NativeAppAction::RequestRedraw => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            NativeAppAction::Exit => event_loop.exit(),
            NativeAppAction::None | NativeAppAction::CreateWindow => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: NativeAppEvent) {
        let mut clipboard = NativeClipboard::new();
        let action = self
            .runtime
            .pump_output_sync_clipboard_and_schedule_redraw(&mut self.lifecycle, &mut clipboard)
            .unwrap_or_else(|error| {
                self.startup_error = Some(error.to_string());
                NativeAppAction::Exit
            });
        let action = match event {
            NativeAppEvent::PtyOutputReady => action,
        };
        match action {
            NativeAppAction::RequestRedraw => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            NativeAppAction::Exit => event_loop.exit(),
            NativeAppAction::None | NativeAppAction::CreateWindow => {}
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                self.lifecycle.on_close_requested();
                event_loop.exit();
            }
            WindowEvent::Destroyed => {
                self.lifecycle.on_destroyed();
                self.surface = None;
                self.gpu_context = None;
                self.window = None;
                self.window_id = None;
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(window) = &self.window {
                    window.pre_present_notify();
                }
                if let Err(error) = self.present_redraw_frame() {
                    match error {
                        NativeGlyphFrameError::Surface(
                            SurfaceFrameError::Timeout | SurfaceFrameError::Occluded,
                        ) => {}
                        NativeGlyphFrameError::Surface(
                            SurfaceFrameError::Outdated
                            | SurfaceFrameError::Lost
                            | SurfaceFrameError::Validation
                            | SurfaceFrameError::InvalidFrame(_),
                        )
                        | NativeGlyphFrameError::Font(_)
                        | NativeGlyphFrameError::Renderer(_) => {
                            self.startup_error = Some(error.to_string());
                            event_loop.exit();
                        }
                    }
                }
                self.lifecycle.on_redraw_requested();
            }
            WindowEvent::Resized(size) => {
                if let Some(surface) = &mut self.surface
                    && let Err(error) = surface.resize(size.width, size.height)
                {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                    return;
                }
                if let Some(resize) = self
                    .resize_mapper
                    .resize_for_window(size.width, size.height)
                    && let Err(error) = self.runtime.resize_terminal(resize)
                {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    let result = if is_native_copy_shortcut(&event.logical_key, self.modifiers) {
                        let mut clipboard = NativeClipboard::new();
                        self.runtime.copy_selection_to_clipboard(&mut clipboard);
                        Ok(())
                    } else if is_native_paste_shortcut(&event.logical_key, self.modifiers) {
                        let clipboard = NativeClipboard::new();
                        self.runtime.send_clipboard_paste(&clipboard).map(|_| ())
                    } else {
                        self.runtime
                            .send_winit_key_event_input(
                                &event.logical_key,
                                Some(event.physical_key),
                                self.modifiers,
                            )
                            .map(|_| ())
                    };
                    if let Err(error) = result {
                        self.startup_error = Some(error.to_string());
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                if let Err(error) = self.runtime.send_committed_text(&text) {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some(position);
                let (kind, button) = self.mouse_buttons.cursor_move_event();
                if let Err(error) = self.send_current_mouse_input(kind, button) {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            WindowEvent::Focused(focused) => {
                if let Err(error) = self.runtime.send_focus_event(focused).map(|_| ()) {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(button) = native_mouse_button(button) {
                    let kind = if state == ElementState::Pressed {
                        self.mouse_buttons.set_pressed(button, true);
                        MouseEventKind::Press
                    } else {
                        MouseEventKind::Release
                    };
                    if let Err(error) = self.send_current_mouse_input(kind, button) {
                        self.startup_error = Some(error.to_string());
                        event_loop.exit();
                    }
                    if state == ElementState::Released {
                        self.mouse_buttons.set_pressed(button, false);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(button) = wheel_mouse_button(delta)
                    && let Err(error) = self.send_current_mouse_input(MouseEventKind::Press, button)
                {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}

impl NativeTerminalApp {
    fn create_surface_for_window(
        &mut self,
        window: Arc<Window>,
        width: u32,
        height: u32,
    ) -> Result<(), NativeAppError> {
        let context = GpuBootstrap::new(GpuBootstrapConfig::native_default())
            .initialize_native()
            .map_err(NativeAppError::from)?;
        let gpu_surface = context
            .create_window_surface(window)
            .map_err(NativeAppError::from)?;
        let surface = NativeWindowSurface::from_gpu_surface(gpu_surface, width, height)
            .map_err(NativeAppError::from)?;
        self.gpu_context = Some(context);
        self.surface = Some(surface);
        Ok(())
    }

    fn send_current_mouse_input(
        &mut self,
        kind: MouseEventKind,
        button: MouseButton,
    ) -> Result<(), NativeAppError> {
        let (Some(position), Some(window)) = (self.cursor_position, self.window.as_ref()) else {
            return Ok(());
        };
        let size = window.inner_size();
        self.runtime
            .send_window_mouse_input_event(NativeWindowMouseInput {
                x: position.x,
                y: position.y,
                window_width_px: size.width,
                window_height_px: size.height,
                kind,
                button,
                modifiers: self.modifiers,
            })
            .map(|_| ())
    }

    fn present_redraw_frame(&mut self) -> Result<(), NativeGlyphFrameError> {
        let Some(surface) = &mut self.surface else {
            self.runtime.render_terminal_frame(&mut self.renderer)?;
            return Ok(());
        };
        if let Some(glyph_cache) = &mut self.glyph_cache {
            render_and_present_terminal_glyph_frame(
                &mut self.runtime,
                &mut self.renderer,
                glyph_cache,
                surface,
            )?;
        } else {
            self.runtime.render_terminal_frame(&mut self.renderer)?;
            surface.clear_and_present(self.renderer.config().clear_color)?;
        }
        Ok(())
    }
}

/// Errors from launching the native application loop.
#[derive(Debug, Error)]
pub enum NativeAppError {
    /// The event loop could not be created or executed.
    #[error("native event loop failed: {0}")]
    EventLoop(#[from] EventLoopError),
    /// The native window could not be created.
    #[error("native window creation failed: {0}")]
    WindowCreation(String),
    /// Native terminal runtime setup failed.
    #[error("native runtime failed: {0}")]
    Runtime(String),
    /// Native GPU setup failed.
    #[error("native GPU setup failed: {0}")]
    Gpu(String),
}

/// Errors while preparing or presenting a terminal glyph frame.
#[derive(Debug, Error)]
pub enum NativeGlyphFrameError {
    /// Font rasterization failed while building the glyph atlas image.
    #[error("native glyph rasterization failed: {0}")]
    Font(#[from] FontRasterError),
    /// Surface frame acquisition, drawing, or presentation failed.
    #[error("native glyph surface presentation failed: {0}")]
    Surface(#[from] SurfaceFrameError),
    /// CPU-side render planning failed before presentation.
    #[error("native glyph render planning failed: {0}")]
    Renderer(#[from] GromaqError),
}

impl From<OsError> for NativeAppError {
    fn from(value: OsError) -> Self {
        Self::WindowCreation(value.to_string())
    }
}

impl From<GromaqError> for NativeAppError {
    fn from(value: GromaqError) -> Self {
        Self::Runtime(value.to_string())
    }
}

impl From<GpuBootstrapError> for NativeAppError {
    fn from(value: GpuBootstrapError) -> Self {
        Self::Gpu(value.to_string())
    }
}

impl From<GpuSurfaceError> for NativeAppError {
    fn from(value: GpuSurfaceError) -> Self {
        Self::Gpu(value.to_string())
    }
}

impl From<SurfaceConfigError> for NativeAppError {
    fn from(value: SurfaceConfigError) -> Self {
        Self::Gpu(value.to_string())
    }
}

impl From<SurfaceFrameError> for NativeAppError {
    fn from(value: SurfaceFrameError) -> Self {
        Self::Gpu(value.to_string())
    }
}

impl From<FontRasterError> for NativeAppError {
    fn from(value: FontRasterError) -> Self {
        Self::Runtime(value.to_string())
    }
}

impl From<NativeGlyphFrameError> for NativeAppError {
    fn from(value: NativeGlyphFrameError) -> Self {
        match value {
            NativeGlyphFrameError::Font(error) => Self::Runtime(error.to_string()),
            NativeGlyphFrameError::Surface(error) => Self::Gpu(error.to_string()),
            NativeGlyphFrameError::Renderer(error) => Self::Runtime(error.to_string()),
        }
    }
}

/// Run the native `winit` terminal application loop.
pub fn run_native_app(config: NativeAppConfig) -> Result<(), NativeAppError> {
    run_native_app_with_runtime_config(config, NativeTerminalRuntimeConfig::default())
}

/// Run the native `winit` terminal application loop with explicit runtime configuration.
pub fn run_native_app_with_runtime_config(
    config: NativeAppConfig,
    runtime_config: NativeTerminalRuntimeConfig,
) -> Result<(), NativeAppError> {
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
) -> Result<(), NativeAppError> {
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
) -> Result<(), NativeAppError> {
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
    Ok(())
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
}
