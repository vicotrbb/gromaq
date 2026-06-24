use crate::TerminalConfig;
use crate::config::GromaqConfig;
use crate::pty::{PtyConfig, PtyError, PtySession, ShellCommand};

use super::{NativeAppError, NativeAppEvent, NativeAppEventProxy, NativePtyResize};

/// Native terminal runtime configuration shared by the app and PTY boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTerminalRuntimeConfig {
    /// Startup terminal columns.
    pub terminal_cols: u16,
    /// Startup terminal rows.
    pub terminal_rows: u16,
    /// Scrollback line limit.
    pub scrollback_lines: usize,
    /// PTY pixel width, if known.
    pub pixel_width: u16,
    /// PTY pixel height, if known.
    pub pixel_height: u16,
    /// Shell command to spawn.
    pub shell: ShellCommand,
}

impl Default for NativeTerminalRuntimeConfig {
    fn default() -> Self {
        let config = GromaqConfig::default();
        Self {
            terminal_cols: config.terminal.cols,
            terminal_rows: config.terminal.rows,
            scrollback_lines: config.terminal.scrollback_lines,
            pixel_width: 0,
            pixel_height: 0,
            shell: ShellCommand::default_shell(),
        }
    }
}

impl NativeTerminalRuntimeConfig {
    /// Build runtime configuration from validated user configuration and shell command.
    pub fn from_gromaq_config(
        config: &GromaqConfig,
        shell: ShellCommand,
    ) -> Result<Self, NativeAppError> {
        config
            .validate()
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        let runtime_config = Self {
            terminal_cols: config.terminal.cols,
            terminal_rows: config.terminal.rows,
            scrollback_lines: config.terminal.scrollback_lines,
            pixel_width: 0,
            pixel_height: 0,
            shell,
        };
        runtime_config.terminal_config()?;
        Ok(runtime_config)
    }

    pub(super) fn terminal_config(&self) -> Result<TerminalConfig, NativeAppError> {
        TerminalConfig::new(self.terminal_cols, self.terminal_rows)
            .and_then(|config| config.with_pixel_size(self.pixel_width, self.pixel_height))
            .and_then(|config| config.with_scrollback_limit(self.scrollback_lines))
            .map_err(|error| NativeAppError::Runtime(error.to_string()))
    }

    pub(super) fn pty_config(&self) -> PtyConfig {
        PtyConfig {
            rows: self.terminal_rows,
            cols: self.terminal_cols,
            pixel_width: self.pixel_width,
            pixel_height: self.pixel_height,
            shell: self.shell.clone(),
        }
    }
}

/// Spawns PTY sessions for the native terminal runtime.
pub trait NativePtySpawner {
    /// Session handle kept alive by the runtime.
    type Session;

    /// Spawn a PTY session with `config`.
    fn spawn(&self, config: PtyConfig) -> Result<Self::Session, PtyError>;
}

/// Minimal I/O surface the native runtime needs from a live PTY session.
pub trait NativePtySessionIo {
    /// Drain currently available PTY output bytes without blocking.
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError>;

    /// Write terminal input bytes to the PTY.
    fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError>;

    /// Resize the PTY backing the terminal session.
    fn resize(&mut self, size: NativePtyResize) -> Result<(), PtyError>;
}

/// Real native PTY spawner.
#[derive(Debug, Clone, Default)]
pub struct RealNativePtySpawner {
    event_proxy: Option<NativeAppEventProxy>,
}

impl RealNativePtySpawner {
    /// Build a real PTY spawner that wakes the native event loop when output arrives.
    pub fn with_event_proxy(event_proxy: NativeAppEventProxy) -> Self {
        Self {
            event_proxy: Some(event_proxy),
        }
    }
}

impl NativePtySpawner for RealNativePtySpawner {
    type Session = PtySession;

    fn spawn(&self, config: PtyConfig) -> Result<Self::Session, PtyError> {
        let mut session = PtySession::spawn(config)?;
        if let Some(event_proxy) = self.event_proxy.clone() {
            session.start_output_reader_with_wakeup(move || {
                event_proxy.send(NativeAppEvent::PtyOutputReady);
            })?;
        } else {
            session.start_output_reader()?;
        }
        Ok(session)
    }
}

impl NativePtySessionIo for PtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        self.drain_available_output()
    }

    fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.write_all(bytes)
    }

    fn resize(&mut self, size: NativePtyResize) -> Result<(), PtyError> {
        PtySession::resize(
            self,
            portable_pty::PtySize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: size.pixel_width,
                pixel_height: size.pixel_height,
            },
        )
    }
}
