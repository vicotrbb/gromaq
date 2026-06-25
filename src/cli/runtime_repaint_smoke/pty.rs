use std::collections::VecDeque;

use crate::app::{NativePtyResize, NativePtySessionIo, NativePtySpawner};
use crate::pty::{PtyConfig, PtyError};

#[derive(Debug, Clone)]
pub(super) struct RepaintSmokePtySpawner {
    payloads: Vec<Vec<u8>>,
}

#[derive(Debug)]
pub(super) struct RepaintSmokePtySession {
    output: VecDeque<Vec<u8>>,
}

impl RepaintSmokePtySpawner {
    pub(super) fn new(payloads: Vec<Vec<u8>>) -> Self {
        Self { payloads }
    }
}

impl NativePtySpawner for RepaintSmokePtySpawner {
    type Session = RepaintSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RepaintSmokePtySession {
            output: VecDeque::from(self.payloads.clone()),
        })
    }
}

impl NativePtySessionIo for RepaintSmokePtySession {
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
