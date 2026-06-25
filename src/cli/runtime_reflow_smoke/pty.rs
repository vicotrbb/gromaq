use std::collections::VecDeque;

use crate::app::{NativePtyResize, NativePtySessionIo, NativePtySpawner};
use crate::pty::{PtyConfig, PtyError};

#[derive(Debug, Clone)]
pub(super) struct RuntimeReflowSmokePtySpawner {
    payload: Vec<u8>,
}

#[derive(Debug)]
pub(super) struct RuntimeReflowSmokePtySession {
    output: VecDeque<Vec<u8>>,
    pub(super) resizes: Vec<NativePtyResize>,
}

impl RuntimeReflowSmokePtySpawner {
    pub(super) fn new(payload: Vec<u8>) -> Self {
        Self { payload }
    }
}

impl NativePtySpawner for RuntimeReflowSmokePtySpawner {
    type Session = RuntimeReflowSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeReflowSmokePtySession {
            output: VecDeque::from([self.payload.clone()]),
            resizes: Vec::new(),
        })
    }
}

impl NativePtySessionIo for RuntimeReflowSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, _bytes: &[u8]) -> Result<(), PtyError> {
        Ok(())
    }

    fn resize(&mut self, size: NativePtyResize) -> Result<(), PtyError> {
        self.resizes.push(size);
        Ok(())
    }
}
