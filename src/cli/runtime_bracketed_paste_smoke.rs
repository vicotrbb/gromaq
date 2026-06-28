//! Runtime bracketed-paste CLI smoke command.

use std::collections::VecDeque;

use super::CliExit;
use crate::app::{
    NativePtyResize, NativePtySessionIo, NativePtySpawner, NativeTerminalRuntime,
    NativeTerminalRuntimeConfig,
};
use crate::pty::{PtyConfig, PtyError, ShellCommand};

const RUNTIME_BRACKETED_PASTE_SMOKE_TEXT: &str = "alpha\nbeta\t界";
const BRACKETED_PASTE_ENABLE: &[u8] = b"\x1b[?2004h";
const BRACKETED_PASTE_PREFIX: &[u8] = b"\x1b[200~";
const BRACKETED_PASTE_SUFFIX: &[u8] = b"\x1b[201~";

#[derive(Debug, Clone, Copy, Default)]
struct RuntimeBracketedPasteSmokePtySpawner;

#[derive(Debug)]
struct RuntimeBracketedPasteSmokePtySession {
    output: VecDeque<Vec<u8>>,
    input: Vec<Vec<u8>>,
}

impl Default for RuntimeBracketedPasteSmokePtySession {
    fn default() -> Self {
        Self {
            output: VecDeque::from([BRACKETED_PASTE_ENABLE.to_vec()]),
            input: Vec::new(),
        }
    }
}

impl NativePtySpawner for RuntimeBracketedPasteSmokePtySpawner {
    type Session = RuntimeBracketedPasteSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeBracketedPasteSmokePtySession::default())
    }
}

impl NativePtySessionIo for RuntimeBracketedPasteSmokePtySession {
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

pub(super) fn runtime_bracketed_paste_smoke_exit() -> CliExit {
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 24,
        terminal_rows: 4,
        scrollback_lines: 128,
        pixel_width: 0,
        pixel_height: 0,
        cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
        cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_bracketed_paste_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&RuntimeBracketedPasteSmokePtySpawner) {
        return runtime_bracketed_paste_smoke_error(error);
    }
    if let Err(error) = runtime.pump_pty_output() {
        return runtime_bracketed_paste_smoke_error(error);
    }
    if let Err(error) = runtime.send_paste_text(RUNTIME_BRACKETED_PASTE_SMOKE_TEXT) {
        return runtime_bracketed_paste_smoke_error(error);
    }

    let expected_input = expected_bracketed_paste_input();
    let metrics = runtime.dump_runtime_perf_metrics();
    let pasted_bytes = runtime
        .shell_session()
        .and_then(|session| session.input.last())
        .map(Vec::as_slice);
    let bracketed = pasted_bytes == Some(expected_input.as_slice());

    if !bracketed
        || metrics.paste_bytes != RUNTIME_BRACKETED_PASTE_SMOKE_TEXT.len() as u64
        || metrics.pty_input_writes != 1
        || metrics.pty_input_bytes != expected_input.len() as u64
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "runtime bracketed paste smoke failed: encoded paste did not reach the PTY\n"
                .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime bracketed paste smoke: ok\npayload bytes: {}\nencoded bytes: {}\npaste bytes: {}\npty input writes: {}\npty input bytes: {}\nbracketed: {}\n",
            RUNTIME_BRACKETED_PASTE_SMOKE_TEXT.len(),
            expected_input.len(),
            metrics.paste_bytes,
            metrics.pty_input_writes,
            metrics.pty_input_bytes,
            bracketed
        ),
        stderr: String::new(),
    }
}

fn expected_bracketed_paste_input() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(
        BRACKETED_PASTE_PREFIX.len()
            + RUNTIME_BRACKETED_PASTE_SMOKE_TEXT.len()
            + BRACKETED_PASTE_SUFFIX.len(),
    );
    bytes.extend_from_slice(BRACKETED_PASTE_PREFIX);
    bytes.extend_from_slice(RUNTIME_BRACKETED_PASTE_SMOKE_TEXT.as_bytes());
    bytes.extend_from_slice(BRACKETED_PASTE_SUFFIX);
    bytes
}

fn runtime_bracketed_paste_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime bracketed paste smoke failed: {error}\n"),
    }
}
