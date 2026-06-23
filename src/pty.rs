//! PTY process boundary.

use std::ffi::OsString;
use std::io::{ErrorKind, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, ExitStatus, MasterPty, PtySize, native_pty_system};
use thiserror::Error;

/// PTY lifecycle errors.
#[derive(Debug, Error)]
pub enum PtyError {
    /// Error returned by `portable-pty`.
    #[error("pty operation failed: {0}")]
    Backend(String),
    /// Standard I/O failure.
    #[error("pty I/O failed with {kind:?}: {message}")]
    Io {
        /// Standard I/O error kind.
        kind: ErrorKind,
        /// Underlying I/O error message.
        message: String,
    },
    /// Output was not available before the deadline.
    #[error("pty read timed out after {0:?}")]
    ReadTimeout(Duration),
    /// The session reader has already been consumed.
    #[error("pty reader has already been consumed")]
    ReaderAlreadyTaken,
    /// The session writer has already been consumed.
    #[error("pty writer has already been consumed")]
    WriterAlreadyTaken,
    /// Reader thread exited without sending a result.
    #[error("pty reader thread disconnected")]
    ReaderDisconnected,
}

type PtyResult<T> = std::result::Result<T, PtyError>;

impl PtyError {
    fn io(error: std::io::Error) -> Self {
        Self::Io {
            kind: error.kind(),
            message: error.to_string(),
        }
    }
}

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

/// Running PTY session.
pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    reader: Option<Box<dyn Read + Send>>,
    output_receiver: Option<Receiver<std::result::Result<Vec<u8>, PtyError>>>,
    writer: Option<Box<dyn Write + Send>>,
}

impl PtySession {
    /// Spawn a configured shell command in a native PTY.
    pub fn spawn(config: PtyConfig) -> PtyResult<Self> {
        let pty_system = native_system();
        let pair = pty_system
            .openpty(config.size())
            .map_err(|error| PtyError::Backend(error.to_string()))?;
        let command = config.shell.to_command_builder();
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| PtyError::Backend(error.to_string()))?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| PtyError::Backend(error.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| PtyError::Backend(error.to_string()))?;

        Ok(Self {
            master: pair.master,
            child,
            reader: Some(reader),
            output_receiver: None,
            writer: Some(writer),
        })
    }

    /// Resize the native PTY master.
    pub fn resize(&mut self, size: PtySize) -> PtyResult<()> {
        self.master
            .resize(size)
            .map_err(|error| PtyError::Backend(error.to_string()))
    }

    /// Write bytes to the PTY master.
    pub fn write_all(&mut self, bytes: &[u8]) -> PtyResult<()> {
        let writer = self.writer.as_mut().ok_or(PtyError::WriterAlreadyTaken)?;
        writer.write_all(bytes).map_err(PtyError::io)
    }

    /// Start a background reader that streams PTY output chunks to `drain_available_output`.
    pub fn start_output_reader(&mut self) -> PtyResult<()> {
        self.start_output_reader_with_wakeup(|| {})
    }

    /// Start a background reader and call `wakeup` whenever output bytes are queued.
    pub fn start_output_reader_with_wakeup<F>(&mut self, wakeup: F) -> PtyResult<()>
    where
        F: Fn() + Send + 'static,
    {
        if self.output_receiver.is_some() {
            return Ok(());
        }
        let mut reader = self.reader.take().ok_or(PtyError::ReaderAlreadyTaken)?;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut buffer = [0_u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => {
                        if sender.send(Ok(buffer[..read].to_vec())).is_err() {
                            break;
                        }
                        wakeup();
                    }
                    Err(error) if error.kind() == ErrorKind::Interrupted => {}
                    Err(error) => {
                        let _ = sender.send(Err(PtyError::io(error)));
                        break;
                    }
                }
            }
        });
        self.output_receiver = Some(receiver);
        Ok(())
    }

    /// Drain output chunks currently available from the background reader.
    pub fn drain_available_output(&mut self) -> PtyResult<Vec<u8>> {
        let receiver = self
            .output_receiver
            .as_ref()
            .ok_or(PtyError::ReaderAlreadyTaken)?;
        let mut output = Vec::new();
        loop {
            match receiver.try_recv() {
                Ok(Ok(chunk)) => output.extend_from_slice(&chunk),
                Ok(Err(error)) => return Err(error),
                Err(mpsc::TryRecvError::Empty) | Err(mpsc::TryRecvError::Disconnected) => {
                    return Ok(output);
                }
            }
        }
    }

    /// Read all available output until EOF, bounded by `timeout`.
    ///
    /// This consumes the session reader. It is intended for lifecycle tests and short commands.
    pub fn read_to_string_timeout(&mut self, timeout: Duration) -> PtyResult<String> {
        let mut reader = self.reader.take().ok_or(PtyError::ReaderAlreadyTaken)?;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut output = String::new();
            let result = reader
                .read_to_string(&mut output)
                .map(|_| output)
                .map_err(PtyError::io);
            let _ = sender.send(result);
        });

        match receiver.recv_timeout(timeout) {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(error)) => Err(error),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = self.child.kill();
                Err(PtyError::ReadTimeout(timeout))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(PtyError::ReaderDisconnected),
        }
    }

    /// Poll the child process without blocking.
    pub fn try_wait(&mut self) -> PtyResult<Option<ExitStatus>> {
        self.child.try_wait().map_err(PtyError::io)
    }

    /// Wait for the child process to exit until `timeout` elapses.
    pub fn wait_timeout(&mut self, timeout: Duration) -> PtyResult<Option<ExitStatus>> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(Some(status));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pty_io_error_preserves_kind_and_message() {
        let error = PtyError::io(std::io::Error::new(ErrorKind::BrokenPipe, "writer closed"));

        match error {
            PtyError::Io { kind, message } => {
                assert_eq!(kind, ErrorKind::BrokenPipe);
                assert_eq!(message, "writer closed");
            }
            other => panic!("expected PTY I/O error, got {other:?}"),
        }
    }
}
