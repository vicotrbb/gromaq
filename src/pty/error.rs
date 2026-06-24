//! PTY error types.

use std::io::ErrorKind;
use std::time::Duration;

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

/// PTY result alias.
pub type PtyResult<T> = std::result::Result<T, PtyError>;

impl PtyError {
    pub(crate) fn io(error: std::io::Error) -> Self {
        Self::Io {
            kind: error.kind(),
            message: error.to_string(),
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
