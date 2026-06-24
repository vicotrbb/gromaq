//! Native terminal runtime state and PTY/input orchestration.

use std::time::Instant;

use tracing::{debug, trace};
use winit::keyboard::{Key, ModifiersState, PhysicalKey};

use crate::clipboard::HostClipboard;
use crate::error::Result as GromaqResult;
use crate::mouse::{MouseButton, MouseEvent, MouseEventKind};
use crate::pty::ShellCommand;
use crate::renderer::GpuRenderer;
use crate::{SelectionRange, Terminal};

use super::NativeAppError;
use super::lifecycle::{NativeAppAction, NativeAppLifecycle};
use super::native_input::{
    NativeMouseGridMapper, NativePtyResize, NativeWindowMouseInput, ScrollbackKeyDirection,
    native_scrollback_key_direction,
};
use super::perf::{
    NativeRuntimePerfSnapshot, NativeRuntimeStateSnapshot, RuntimeDurationHistogram,
    add_usize_counter, average_duration_nanos, dirty_region_cell_count, saturating_duration_nanos,
    scrollback_cell_count,
};
use super::pty_bridge::{NativePtySessionIo, NativePtySpawner, NativeTerminalRuntimeConfig};

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
