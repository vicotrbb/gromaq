//! PTY shell command and spawn configuration.

use std::ffi::OsString;
use std::path::PathBuf;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

/// Shell launch configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommand {
    /// Program path or name.
    pub program: OsString,
    /// Program arguments.
    pub args: Vec<OsString>,
    /// Working directory.
    pub cwd: Option<PathBuf>,
}

impl ShellCommand {
    /// Build a shell command for the current user's shell, falling back to `sh`.
    pub fn default_shell() -> Self {
        let program = std::env::var_os("SHELL").unwrap_or_else(|| OsString::from("sh"));
        Self {
            program,
            args: Vec::new(),
            cwd: None,
        }
    }

    /// Convert to `portable_pty`'s command builder.
    pub fn to_command_builder(&self) -> CommandBuilder {
        let mut builder = CommandBuilder::new(&self.program);
        for arg in &self.args {
            builder.arg(arg);
        }
        if let Some(cwd) = &self.cwd {
            builder.cwd(cwd);
        }
        builder
    }
}

/// PTY spawn configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyConfig {
    /// Terminal rows.
    pub rows: u16,
    /// Terminal columns.
    pub cols: u16,
    /// Cell width in pixels when known.
    pub pixel_width: u16,
    /// Cell height in pixels when known.
    pub pixel_height: u16,
    /// Shell command.
    pub shell: ShellCommand,
}

impl PtyConfig {
    /// Convert to the size structure used by `portable-pty`.
    pub fn size(&self) -> PtySize {
        PtySize {
            rows: self.rows,
            cols: self.cols,
            pixel_width: self.pixel_width,
            pixel_height: self.pixel_height,
        }
    }
}

/// Create the native PTY system implementation.
pub fn native_system() -> Box<dyn portable_pty::PtySystem + Send> {
    native_pty_system()
}
