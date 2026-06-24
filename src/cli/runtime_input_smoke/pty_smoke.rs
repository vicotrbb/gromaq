use std::collections::VecDeque;

use crate::app::{NativePtyResize, NativePtySessionIo, NativePtySpawner};
use crate::pty::{PtyConfig, PtyError};

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct RuntimePerfSmokePtySpawner;

#[derive(Debug, Default)]
pub(super) struct RuntimePerfSmokePtySession {
    output: VecDeque<Vec<u8>>,
}

impl NativePtySpawner for RuntimePerfSmokePtySpawner {
    type Session = RuntimePerfSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimePerfSmokePtySession::default())
    }
}

impl NativePtySessionIo for RuntimePerfSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.output.push_back(bytes.to_vec());
        Ok(())
    }

    fn resize(&mut self, _size: NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RuntimeInputCaptureSmokePtySpawner {
    output: &'static [u8],
}

#[derive(Debug, Default)]
pub(super) struct RuntimeInputCaptureSmokePtySession {
    output: VecDeque<Vec<u8>>,
    pub(super) input: Vec<Vec<u8>>,
}

impl RuntimeInputCaptureSmokePtySpawner {
    pub(super) const fn new(output: &'static [u8]) -> Self {
        Self { output }
    }
}

impl RuntimeInputCaptureSmokePtySession {
    fn new(output: &'static [u8]) -> Self {
        Self {
            output: VecDeque::from([output.to_vec()]),
            input: Vec::new(),
        }
    }
}

impl NativePtySpawner for RuntimeInputCaptureSmokePtySpawner {
    type Session = RuntimeInputCaptureSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeInputCaptureSmokePtySession::new(self.output))
    }
}

impl NativePtySessionIo for RuntimeInputCaptureSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.input.push(bytes.to_vec());
        Ok(())
    }

    fn resize(&mut self, _size: NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}
