//! Running PTY session lifecycle and I/O.

use std::io::{ErrorKind, Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use portable_pty::{Child, ExitStatus, MasterPty, PtySize};

use super::command::{PtyConfig, native_system};
use super::error::{PtyError, PtyResult};

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
                    Err(error) if is_pty_reader_eof(&error) => break,
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

fn is_pty_reader_eof(error: &std::io::Error) -> bool {
    #[cfg(unix)]
    {
        const UNIX_EIO: i32 = 5;
        error.raw_os_error() == Some(UNIX_EIO)
    }
    #[cfg(not(unix))]
    {
        let _ = error;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pty_reader_treats_unix_eio_as_eof() {
        let error = std::io::Error::from_raw_os_error(5);

        assert!(is_pty_reader_eof(&error));
    }

    #[test]
    fn pty_reader_keeps_other_read_errors_fatal() {
        let error = std::io::Error::new(ErrorKind::BrokenPipe, "writer closed");

        assert!(!is_pty_reader_eof(&error));
    }
}
