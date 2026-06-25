//! Native terminal runtime state and PTY/input orchestration.

use std::time::Instant;

use tracing::{debug, trace};

use crate::clipboard::HostClipboard;
use crate::pty::ShellCommand;
use crate::{SelectionRange, Terminal};

use super::NativeAppError;
use super::lifecycle::{NativeAppAction, NativeAppLifecycle};
use super::native_input::NativePtyResize;
use super::perf::{NativeRuntimePerfSnapshot, RuntimeDurationHistogram, add_usize_counter};
use super::pty_bridge::{NativePtySessionIo, NativePtySpawner, NativeTerminalRuntimeConfig};

mod input;
mod rendering;

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

    /// Write deterministic startup text into the terminal before the native app presents.
    pub fn write_startup_text(&mut self, text: &str) -> Result<(), NativeAppError> {
        self.terminal
            .write_str(text)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))
    }

    /// Access runtime configuration.
    pub fn config(&self) -> &NativeTerminalRuntimeConfig {
        &self.config
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
