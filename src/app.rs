//! Native `winit` application loop boundary.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use thiserror::Error;
use tracing::{debug, trace};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::error::{EventLoopError, OsError};
use winit::event::{ElementState, Ime, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::clipboard::{HostClipboard, NativeClipboard};
use crate::config::{ConfigFileReloader, GromaqConfig};
use crate::error::{GromaqError, Result as GromaqResult};
use crate::font::{FontRasterError, RasterizedGlyphCache};
use crate::mouse::{MouseButton, MouseEvent, MouseEventKind};
use crate::native_gpu::{
    GpuBootstrap, GpuBootstrapConfig, GpuBootstrapError, GpuSurfaceError, NativeGpuContext,
};
use crate::pty::{PtySession, ShellCommand};
use crate::renderer::{
    GpuRenderer, RendererConfig, SurfaceConfigError, SurfaceFrameError, WgpuRenderer,
    WgpuSurfaceBackend,
};
use crate::{SelectionRange, Terminal};

mod lifecycle;
mod native_input;
mod perf;
mod pty_bridge;
mod surface;
pub use lifecycle::{
    NativeAppAction, NativeAppConfig, NativeAppEvent, NativeAppEventProxy, NativeAppLifecycle,
};
pub use native_input::{
    NativeMouseButtonTracker, NativeMouseGridMapper, NativePtyResize, NativeResizeGridMapper,
    NativeWindowMouseInput, is_native_copy_shortcut, is_native_paste_shortcut,
};
use native_input::{
    ScrollbackKeyDirection, clamp_u32_to_u16, native_mouse_button, native_scrollback_key_direction,
    wheel_mouse_button,
};
pub use perf::{NativeRuntimePerfSnapshot, NativeRuntimeStateSnapshot};
use perf::{
    RuntimeDurationHistogram, add_usize_counter, average_duration_nanos, dirty_region_cell_count,
    saturating_duration_nanos, scrollback_cell_count,
};
pub use pty_bridge::{
    NativePtySessionIo, NativePtySpawner, NativeTerminalRuntimeConfig, RealNativePtySpawner,
};
pub use surface::{
    NativeWindowSurface, load_default_native_glyph_cache, render_and_present_terminal_glyph_frame,
};

/// Runtime state owned by the native app after startup.
pub struct NativeTerminalRuntime<S> {
    config: NativeTerminalRuntimeConfig,
    terminal: Terminal,
    shell_session: Option<S>,
    last_synced_clipboard_text: Option<String>,
    perf: NativeRuntimePerfSnapshot,
    render_time_histogram: RuntimeDurationHistogram,
    input_to_render_histogram: RuntimeDurationHistogram,
    pending_input_to_render_started: Option<Instant>,
}

impl<S> NativeTerminalRuntime<S> {
    /// Create runtime state with terminal grid and shell settings.
    pub fn new(config: NativeTerminalRuntimeConfig) -> Result<Self, NativeAppError> {
        let terminal = Terminal::new(config.terminal_config()?);
        debug!(
            terminal_cols = config.terminal_cols,
            terminal_rows = config.terminal_rows,
            scrollback_lines = config.scrollback_lines,
            shell = ?config.shell.program,
            "created native terminal runtime"
        );
        Ok(Self {
            config,
            terminal,
            shell_session: None,
            last_synced_clipboard_text: None,
            perf: NativeRuntimePerfSnapshot::default(),
            render_time_histogram: RuntimeDurationHistogram::default(),
            input_to_render_histogram: RuntimeDurationHistogram::default(),
            pending_input_to_render_started: None,
        })
    }

    /// Access the terminal state.
    pub fn terminal(&self) -> &Terminal {
        &self.terminal
    }

    /// Access runtime configuration.
    pub fn config(&self) -> &NativeTerminalRuntimeConfig {
        &self.config
    }

    /// Return deterministic native runtime counters.
    pub fn dump_runtime_perf_metrics(&self) -> NativeRuntimePerfSnapshot {
        self.perf
    }

    /// Return deterministic runtime state-footprint counters.
    pub fn dump_runtime_state_snapshot(&self) -> NativeRuntimeStateSnapshot {
        let scrollback = self.terminal.dump_scrollback();
        NativeRuntimeStateSnapshot {
            terminal_cols: self.config.terminal_cols,
            terminal_rows: self.config.terminal_rows,
            visible_cells: usize::from(self.config.terminal_cols)
                .saturating_mul(usize::from(self.config.terminal_rows)),
            scrollback_limit: self.config.scrollback_lines,
            scrollback_lines: scrollback.lines.len(),
            scrollback_cell_rows: scrollback.cells.len(),
            scrollback_cells: scrollback_cell_count(&scrollback),
            scrollback_cell_limit: self
                .config
                .scrollback_lines
                .saturating_mul(usize::from(self.config.terminal_cols)),
        }
    }

    /// Render the current terminal frame when dirty regions are pending.
    pub fn render_terminal_frame<R>(&mut self, renderer: &mut R) -> GromaqResult<bool>
    where
        R: GpuRenderer,
    {
        self.perf.render_attempts += 1;
        let dirty_regions = self.terminal.take_dirty_regions();
        if dirty_regions.is_empty() {
            self.perf.clean_frame_skips += 1;
            trace!(
                render_attempts = self.perf.render_attempts,
                clean_frame_skips = self.perf.clean_frame_skips,
                "skipped clean native terminal frame"
            );
            return Ok(false);
        }
        let render_started = Instant::now();
        if let Err(error) = renderer.render_frame(
            &self.terminal.dump_grid(),
            self.terminal.dump_cursor(),
            &dirty_regions,
        ) {
            self.terminal.invalidate_viewport();
            return Err(error);
        }
        let elapsed_ns = saturating_duration_nanos(render_started.elapsed());
        let dirty_cells = dirty_region_cell_count(&dirty_regions);
        self.perf.rendered_frames += 1;
        add_usize_counter(&mut self.perf.rendered_dirty_regions, dirty_regions.len());
        self.perf.rendered_dirty_cells = self.perf.rendered_dirty_cells.saturating_add(dirty_cells);
        self.perf.rendered_dirty_cells_max = self.perf.rendered_dirty_cells_max.max(dirty_cells);
        self.perf.render_time_samples += 1;
        self.perf.render_time_total_ns = self.perf.render_time_total_ns.saturating_add(elapsed_ns);
        self.perf.render_time_avg_ns = average_duration_nanos(
            self.perf.render_time_total_ns,
            self.perf.render_time_samples,
        );
        self.perf.render_time_max_ns = self.perf.render_time_max_ns.max(elapsed_ns);
        self.render_time_histogram.record(elapsed_ns);
        self.perf.render_time_p95_ns = self
            .render_time_histogram
            .p95_upper_bound_ns(self.perf.render_time_samples);
        trace!(
            dirty_regions = dirty_regions.len(),
            dirty_cells,
            render_time_ns = elapsed_ns,
            rendered_frames = self.perf.rendered_frames,
            render_time_p95_ns = self.perf.render_time_p95_ns,
            "rendered native terminal frame"
        );
        if let Some(input_started) = self.pending_input_to_render_started.take() {
            self.record_input_to_render_latency(saturating_duration_nanos(input_started.elapsed()));
        }
        Ok(true)
    }

    /// Force the next renderer pass to cover the visible terminal viewport.
    pub fn invalidate_terminal_frame(&mut self) {
        self.terminal.invalidate_viewport();
    }

    fn record_input_to_render_latency(&mut self, elapsed_ns: u64) {
        self.perf.input_to_render_samples += 1;
        self.perf.input_to_render_total_ns = self
            .perf
            .input_to_render_total_ns
            .saturating_add(elapsed_ns);
        self.perf.input_to_render_avg_ns = average_duration_nanos(
            self.perf.input_to_render_total_ns,
            self.perf.input_to_render_samples,
        );
        self.perf.input_to_render_max_ns = self.perf.input_to_render_max_ns.max(elapsed_ns);
        self.input_to_render_histogram.record(elapsed_ns);
        self.perf.input_to_render_p95_ns = self
            .input_to_render_histogram
            .p95_upper_bound_ns(self.perf.input_to_render_samples);
    }

    /// Write terminal-owned clipboard text, such as OSC 52 payloads, to a host clipboard.
    pub fn sync_terminal_clipboard<C>(&mut self, clipboard: &mut C) -> bool
    where
        C: HostClipboard,
    {
        let Some(text) = self.terminal.dump_clipboard_text() else {
            return false;
        };
        if self.last_synced_clipboard_text.as_deref() == Some(text.as_str()) {
            return false;
        }
        clipboard.write_text(&text);
        self.last_synced_clipboard_text = Some(text);
        true
    }

    /// Set the active visible-grid selection.
    pub fn set_selection(&mut self, selection: SelectionRange) {
        self.terminal.set_selection(selection);
    }

    /// Copy the active terminal selection into a host clipboard adapter.
    pub fn copy_selection_to_clipboard<C>(&self, clipboard: &mut C) -> bool
    where
        C: HostClipboard,
    {
        self.terminal
            .copy_selection_to_clipboard(clipboard)
            .is_some()
    }

    /// Access the retained shell session.
    pub fn shell_session(&self) -> Option<&S> {
        self.shell_session.as_ref()
    }

    /// Whether a shell session has been attached.
    pub fn has_shell_session(&self) -> bool {
        self.shell_session.is_some()
    }

    /// Start the shell PTY once and retain the live session handle.
    pub fn start_shell<P>(&mut self, spawner: &P) -> Result<(), NativeAppError>
    where
        P: NativePtySpawner<Session = S>,
    {
        if self.shell_session.is_some() {
            return Ok(());
        }
        let pty_config = self.config.pty_config();
        debug!(
            cols = pty_config.cols,
            rows = pty_config.rows,
            pixel_width = pty_config.pixel_width,
            pixel_height = pty_config.pixel_height,
            shell = ?pty_config.shell.program,
            "starting native shell PTY"
        );
        let session = spawner
            .spawn(pty_config)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        self.shell_session = Some(session);
        debug!("native shell PTY started");
        Ok(())
    }

    /// Update the shell command used for the next PTY spawn.
    pub fn set_shell_command(&mut self, shell: ShellCommand) {
        self.config.shell = shell;
    }

    /// Replace the configured shell and start a fresh PTY session.
    pub fn restart_shell<P>(
        &mut self,
        shell: ShellCommand,
        spawner: &P,
    ) -> Result<(), NativeAppError>
    where
        P: NativePtySpawner<Session = S>,
    {
        self.config.shell = shell;
        let pty_config = self.config.pty_config();
        debug!(
            cols = pty_config.cols,
            rows = pty_config.rows,
            pixel_width = pty_config.pixel_width,
            pixel_height = pty_config.pixel_height,
            shell = ?pty_config.shell.program,
            "restarting native shell PTY"
        );
        let session = spawner
            .spawn(pty_config)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        self.shell_session = Some(session);
        debug!("native shell PTY restarted");
        Ok(())
    }
}

impl<S> NativeTerminalRuntime<S>
where
    S: NativePtySessionIo,
{
    /// Drain available PTY output and feed it into the terminal parser.
    pub fn pump_pty_output(&mut self) -> Result<usize, NativeAppError> {
        let Some(session) = self.shell_session.as_mut() else {
            return Ok(0);
        };
        let output = session
            .drain_output()
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        if output.is_empty() {
            return Ok(0);
        }
        self.perf.pty_output_batches += 1;
        add_usize_counter(&mut self.perf.pty_output_bytes, output.len());
        self.terminal
            .write_bytes(&output)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        let response = self.terminal.take_pending_response_bytes();
        if !response.is_empty() {
            session
                .write_input(&response)
                .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
            self.perf.pty_response_writes += 1;
            add_usize_counter(&mut self.perf.pty_response_bytes, response.len());
        }
        trace!(
            output_bytes = output.len(),
            response_bytes = response.len(),
            output_batches = self.perf.pty_output_batches,
            total_output_bytes = self.perf.pty_output_bytes,
            "pumped native PTY output"
        );
        Ok(output.len())
    }

    /// Write encoded terminal input bytes to the PTY session.
    pub fn send_pty_input(&mut self, bytes: &[u8]) -> Result<(), NativeAppError> {
        if bytes.is_empty() {
            return Ok(());
        }
        let Some(session) = self.shell_session.as_mut() else {
            return Ok(());
        };
        session
            .write_input(bytes)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        self.perf.pty_input_writes += 1;
        add_usize_counter(&mut self.perf.pty_input_bytes, bytes.len());
        trace!(
            input_bytes = bytes.len(),
            input_writes = self.perf.pty_input_writes,
            total_input_bytes = self.perf.pty_input_bytes,
            "wrote native PTY input"
        );
        if self.pending_input_to_render_started.is_none() {
            self.pending_input_to_render_started = Some(Instant::now());
        }
        Ok(())
    }

    /// Encode a native logical key and write it to the PTY when it maps to terminal input.
    pub fn send_winit_key_input(
        &mut self,
        key: &Key,
        modifiers: ModifiersState,
    ) -> Result<bool, NativeAppError> {
        self.send_winit_key_event_input(key, None, modifiers)
    }

    /// Encode a native key event and write it to the PTY when it maps to terminal input.
    pub fn send_winit_key_event_input(
        &mut self,
        key: &Key,
        physical_key: Option<PhysicalKey>,
        modifiers: ModifiersState,
    ) -> Result<bool, NativeAppError> {
        if let Some(direction) = native_scrollback_key_direction(key, modifiers) {
            let alternate_screen_active = self.terminal.is_alternate_screen_active();
            let rows = self.terminal.dump_grid().rows.saturating_sub(1).max(1);
            if match direction {
                ScrollbackKeyDirection::Up => self.terminal.scroll_display_up(rows),
                ScrollbackKeyDirection::Down => self.terminal.scroll_display_down(rows),
            } {
                return Ok(true);
            }
            if !alternate_screen_active {
                return Ok(false);
            }
        }

        let Some(bytes) = self
            .terminal
            .encode_winit_key_event_input(key, physical_key, modifiers)
        else {
            return Ok(false);
        };
        let had_session = self.shell_session.is_some();
        self.send_pty_input(&bytes)?;
        if had_session {
            self.perf.native_key_inputs += 1;
        }
        Ok(true)
    }

    /// Encode a terminal mouse event and write it to the PTY when reporting is enabled.
    pub fn send_mouse_input(&mut self, event: MouseEvent) -> Result<bool, NativeAppError> {
        let Some(bytes) = self.terminal.encode_mouse_event(event) else {
            return Ok(false);
        };
        let had_session = self.shell_session.is_some();
        self.send_pty_input(&bytes)?;
        if had_session {
            self.perf.mouse_inputs += 1;
        }
        Ok(true)
    }

    /// Encode a terminal focus event and write it to the PTY when reporting is enabled.
    pub fn send_focus_event(&mut self, focused: bool) -> Result<bool, NativeAppError> {
        let Some(bytes) = self.terminal.encode_focus_event(focused) else {
            return Ok(false);
        };
        let had_session = self.shell_session.is_some();
        self.send_pty_input(&bytes)?;
        if had_session {
            self.perf.focus_inputs += 1;
        }
        Ok(true)
    }

    /// Map a native window mouse position to a terminal event and write its report to the PTY.
    pub fn send_window_mouse_input(
        &mut self,
        x: f64,
        y: f64,
        window_width_px: u32,
        window_height_px: u32,
        kind: MouseEventKind,
        button: MouseButton,
    ) -> Result<bool, NativeAppError> {
        self.send_window_mouse_input_event(NativeWindowMouseInput {
            x,
            y,
            window_width_px,
            window_height_px,
            kind,
            button,
            modifiers: ModifiersState::empty(),
        })
    }

    /// Map native window mouse input to a terminal event and write its report.
    pub fn send_window_mouse_input_event(
        &mut self,
        input: NativeWindowMouseInput,
    ) -> Result<bool, NativeAppError> {
        let grid = self.terminal.dump_grid();
        let Some(mapper) = NativeMouseGridMapper::new(
            input.window_width_px,
            input.window_height_px,
            grid.cols,
            grid.rows,
        ) else {
            return Ok(false);
        };
        let Some(event) = mapper.mouse_event_at_with_modifiers(
            input.x,
            input.y,
            input.kind,
            input.button,
            input.modifiers,
        ) else {
            return Ok(false);
        };
        if self.send_mouse_input(event)? {
            return Ok(true);
        }
        Ok(match (input.kind, input.button) {
            (MouseEventKind::Press, MouseButton::WheelUp) => self.terminal.scroll_display_up(1),
            (MouseEventKind::Press, MouseButton::WheelDown) => self.terminal.scroll_display_down(1),
            _ => false,
        })
    }

    /// Encode pasted text according to terminal mode and write it to the PTY.
    pub fn send_paste_text(&mut self, text: &str) -> Result<(), NativeAppError> {
        let bytes = self.terminal.encode_paste_text(text);
        let had_session = self.shell_session.is_some();
        self.send_pty_input(&bytes)?;
        if had_session {
            add_usize_counter(&mut self.perf.paste_bytes, text.len());
        }
        Ok(())
    }

    /// Read text from a host clipboard and write it to the PTY as a terminal paste.
    pub fn send_clipboard_paste<C>(&mut self, clipboard: &C) -> Result<bool, NativeAppError>
    where
        C: HostClipboard,
    {
        if self.shell_session.is_none() {
            return Ok(false);
        }
        let Some(text) = clipboard.read_text().filter(|text| !text.is_empty()) else {
            return Ok(false);
        };
        self.send_paste_text(&text)?;
        self.perf.clipboard_pastes += 1;
        Ok(true)
    }

    /// Write committed platform text input to the PTY as typed UTF-8 text.
    pub fn send_committed_text(&mut self, text: &str) -> Result<(), NativeAppError> {
        let had_session = self.shell_session.is_some();
        self.send_pty_input(text.as_bytes())?;
        if had_session {
            add_usize_counter(&mut self.perf.committed_text_bytes, text.len());
        }
        Ok(())
    }

    /// Resize terminal state and notify the retained PTY session.
    pub fn resize_terminal(&mut self, size: NativePtyResize) -> Result<(), NativeAppError> {
        self.terminal
            .resize_with_pixel_size(size.cols, size.rows, size.pixel_width, size.pixel_height)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        self.config.terminal_cols = size.cols;
        self.config.terminal_rows = size.rows;
        self.config.pixel_width = size.pixel_width;
        self.config.pixel_height = size.pixel_height;
        if let Some(session) = self.shell_session.as_mut() {
            session
                .resize(size)
                .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        }
        self.perf.resize_events += 1;
        Ok(())
    }

    /// Reconfigure terminal dimensions, pixel size, and scrollback retention without restarting the PTY.
    pub fn reconfigure_terminal(
        &mut self,
        config: NativeTerminalRuntimeConfig,
    ) -> Result<(), NativeAppError> {
        self.terminal
            .reconfigure(config.terminal_config()?)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        let resize = NativePtyResize {
            cols: config.terminal_cols,
            rows: config.terminal_rows,
            pixel_width: config.pixel_width,
            pixel_height: config.pixel_height,
        };
        if let Some(session) = self.shell_session.as_mut() {
            session
                .resize(resize)
                .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        }
        self.config = config;
        self.perf.resize_events += 1;
        Ok(())
    }

    /// Drain PTY output at the event-loop idle boundary and request redraw only when output changed.
    pub fn pump_output_and_schedule_redraw(
        &mut self,
        lifecycle: &mut NativeAppLifecycle,
    ) -> Result<NativeAppAction, NativeAppError> {
        let pumped_bytes = self.pump_pty_output()?;
        if pumped_bytes > 0 {
            Ok(lifecycle.on_terminal_output_ready())
        } else {
            Ok(lifecycle.on_about_to_wait())
        }
    }

    /// Drain PTY output, sync terminal clipboard state, and request redraw only when output changed.
    pub fn pump_output_sync_clipboard_and_schedule_redraw<C>(
        &mut self,
        lifecycle: &mut NativeAppLifecycle,
        clipboard: &mut C,
    ) -> Result<NativeAppAction, NativeAppError>
    where
        C: HostClipboard,
    {
        let pumped_bytes = self.pump_pty_output()?;
        if pumped_bytes > 0 {
            self.sync_terminal_clipboard(clipboard);
            Ok(lifecycle.on_terminal_output_ready())
        } else {
            Ok(lifecycle.on_about_to_wait())
        }
    }
}

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
