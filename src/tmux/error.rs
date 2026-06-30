//! Error types for native tmux integration.

use std::io;

use super::runner::TmuxCommandFailure;

/// Errors returned by tmux probing, parsing, and action helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmuxError {
    /// The `tmux` binary is not available on PATH.
    Missing,
    /// A tmux command exited unsuccessfully.
    Command(TmuxCommandFailure),
    /// A tmux command could not be started or waited on.
    Io(String),
    /// tmux output did not match the stable parser contract.
    Parse {
        /// Parser surface that failed.
        context: &'static str,
        /// Offending row or value.
        row: String,
    },
    /// A tmux workspace preset is not launchable.
    InvalidWorkspace {
        /// Workspace key or session name.
        workspace: String,
        /// Validation failure.
        reason: &'static str,
    },
}

impl From<io::Error> for TmuxError {
    fn from(error: io::Error) -> Self {
        if error.kind() == io::ErrorKind::NotFound {
            return Self::Missing;
        }
        Self::Io(error.to_string())
    }
}
