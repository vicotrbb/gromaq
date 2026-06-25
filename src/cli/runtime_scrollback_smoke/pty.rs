use std::collections::VecDeque;

use crate::app::{NativePtyResize, NativePtySessionIo, NativePtySpawner};
use crate::pty::{PtyConfig, PtyError};

use super::RUNTIME_SCROLLBACK_SMOKE_TEXT;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct RuntimeScrollbackSmokePtySpawner;

#[derive(Debug)]
pub(super) struct RuntimeScrollbackSmokePtySession {
    output: VecDeque<Vec<u8>>,
}

impl NativePtySpawner for RuntimeScrollbackSmokePtySpawner {
    type Session = RuntimeScrollbackSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeScrollbackSmokePtySession {
            output: VecDeque::from([RUNTIME_SCROLLBACK_SMOKE_TEXT.as_bytes().to_vec()]),
        })
    }
}

impl NativePtySessionIo for RuntimeScrollbackSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, _bytes: &[u8]) -> Result<(), PtyError> {
        Ok(())
    }

    fn resize(&mut self, _size: NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}
