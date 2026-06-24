use std::collections::VecDeque;

use crate::app::{NativePtyResize, NativePtySessionIo, NativePtySpawner};
use crate::pty::{PtyConfig, PtyError};

#[derive(Debug, Clone)]
pub(super) struct RuntimeLargeOutputSmokePtySpawner {
    payload: Vec<u8>,
}

#[derive(Debug)]
pub(super) struct RuntimeLargeOutputSmokePtySession {
    output: VecDeque<Vec<u8>>,
}

impl RuntimeLargeOutputSmokePtySpawner {
    pub(super) fn new(payload: Vec<u8>) -> Self {
        Self { payload }
    }
}

impl NativePtySpawner for RuntimeLargeOutputSmokePtySpawner {
    type Session = RuntimeLargeOutputSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeLargeOutputSmokePtySession {
            output: VecDeque::from([self.payload.clone()]),
        })
    }
}

impl NativePtySessionIo for RuntimeLargeOutputSmokePtySession {
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

#[derive(Debug, Clone)]
pub(super) struct RuntimeChunkedOutputSmokePtySpawner {
    payloads: Vec<Vec<u8>>,
}

#[derive(Debug)]
pub(super) struct RuntimeChunkedOutputSmokePtySession {
    output: VecDeque<Vec<u8>>,
}

impl RuntimeChunkedOutputSmokePtySpawner {
    pub(super) fn new(payloads: Vec<Vec<u8>>) -> Self {
        Self { payloads }
    }
}

impl NativePtySpawner for RuntimeChunkedOutputSmokePtySpawner {
    type Session = RuntimeChunkedOutputSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeChunkedOutputSmokePtySession {
            output: VecDeque::from(self.payloads.clone()),
        })
    }
}

impl NativePtySessionIo for RuntimeChunkedOutputSmokePtySession {
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
