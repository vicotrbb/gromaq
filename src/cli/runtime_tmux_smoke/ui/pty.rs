//! No-op PTY adapter for native tmux UI runtime smoke.

use crate::app::{NativePtyResize, NativePtySessionIo};
use crate::pty::PtyError;

#[derive(Debug)]
pub(super) struct TmuxUiSmokePtySession;

impl NativePtySessionIo for TmuxUiSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(Vec::new())
    }

    fn write_input(&mut self, _bytes: &[u8]) -> Result<(), PtyError> {
        Ok(())
    }

    fn resize(&mut self, _size: NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}
