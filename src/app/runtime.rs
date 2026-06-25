//! Native terminal runtime state and PTY/input orchestration.

use std::time::Instant;

use tracing::debug;

use crate::clipboard::HostClipboard;
use crate::pty::ShellCommand;
use crate::{SelectionRange, Terminal};

use super::NativeAppError;
use super::perf::{NativeRuntimePerfSnapshot, RuntimeDurationHistogram};
use super::pty_bridge::{NativePtySpawner, NativeTerminalRuntimeConfig};

mod input;
mod pty;
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
