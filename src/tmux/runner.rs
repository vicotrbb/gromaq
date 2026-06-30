//! tmux command execution boundary.

use std::process::Command;

use super::error::TmuxError;

/// Successful tmux command output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxCommandOutput {
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
}

impl TmuxCommandOutput {
    /// Build command output from UTF-8 text.
    pub fn new(stdout: String, stderr: String) -> Self {
        Self { stdout, stderr }
    }
}

/// Failed tmux command details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxCommandFailure {
    /// tmux arguments used for the failed command.
    pub args: Vec<String>,
    /// Process exit code when available.
    pub exit_code: Option<i32>,
    /// Captured standard error.
    pub stderr: String,
}

impl TmuxCommandFailure {
    /// Build a failed command record.
    pub fn new(args: Vec<String>, exit_code: i32, stderr: String) -> Self {
        Self {
            args,
            exit_code: Some(exit_code),
            stderr,
        }
    }
}

/// Command runner abstraction used to test tmux behavior without real tmux.
pub trait TmuxCommandRunner {
    /// Run `tmux` with the provided arguments.
    fn run_tmux(&self, args: &[&str]) -> Result<TmuxCommandOutput, TmuxError>;
}

impl<T> TmuxCommandRunner for &T
where
    T: TmuxCommandRunner + ?Sized,
{
    fn run_tmux(&self, args: &[&str]) -> Result<TmuxCommandOutput, TmuxError> {
        (*self).run_tmux(args)
    }
}

/// Real system `tmux` command runner.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemTmuxCommandRunner;

impl TmuxCommandRunner for SystemTmuxCommandRunner {
    fn run_tmux(&self, args: &[&str]) -> Result<TmuxCommandOutput, TmuxError> {
        let output = Command::new("tmux").args(args).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if output.status.success() {
            return Ok(TmuxCommandOutput { stdout, stderr });
        }
        Err(TmuxError::Command(TmuxCommandFailure {
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            exit_code: output.status.code(),
            stderr,
        }))
    }
}

/// Real system tmux runner scoped to an isolated socket name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketTmuxCommandRunner {
    socket_name: String,
}

impl SocketTmuxCommandRunner {
    /// Create a tmux runner that prepends `-L <socket-name>`.
    pub fn new(socket_name: impl Into<String>) -> Self {
        Self {
            socket_name: socket_name.into(),
        }
    }

    /// Return the tmux socket name.
    pub fn socket_name(&self) -> &str {
        &self.socket_name
    }
}

impl TmuxCommandRunner for SocketTmuxCommandRunner {
    fn run_tmux(&self, args: &[&str]) -> Result<TmuxCommandOutput, TmuxError> {
        let mut socket_args = vec!["-L", self.socket_name.as_str()];
        socket_args.extend_from_slice(args);
        let output = Command::new("tmux").args(&socket_args).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if output.status.success() {
            return Ok(TmuxCommandOutput { stdout, stderr });
        }
        Err(TmuxError::Command(TmuxCommandFailure {
            args: socket_args.iter().map(|arg| (*arg).to_owned()).collect(),
            exit_code: output.status.code(),
            stderr,
        }))
    }
}
