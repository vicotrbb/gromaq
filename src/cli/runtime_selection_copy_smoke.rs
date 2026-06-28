//! Runtime selection-copy CLI smoke command.

use std::collections::VecDeque;

use super::CliExit;
use crate::SelectionRange;
use crate::app::{
    NativePtyResize, NativePtySessionIo, NativePtySpawner, NativeTerminalRuntime,
    NativeTerminalRuntimeConfig,
};
use crate::clipboard::HostClipboard;
use crate::pty::{PtyConfig, PtyError, ShellCommand};

const RUNTIME_SELECTION_COPY_SMOKE_OUTPUT: &[u8] = b"alpha!\r\nbeta\xe7\x95\x8c\r\nready";
const RUNTIME_SELECTION_COPY_SMOKE_TEXT: &str = "alpha!\nbeta界";

#[derive(Debug, Clone, Copy, Default)]
struct RuntimeSelectionCopySmokePtySpawner;

#[derive(Debug)]
struct RuntimeSelectionCopySmokePtySession {
    output: VecDeque<Vec<u8>>,
    input: Vec<Vec<u8>>,
}

impl Default for RuntimeSelectionCopySmokePtySession {
    fn default() -> Self {
        Self {
            output: VecDeque::from([RUNTIME_SELECTION_COPY_SMOKE_OUTPUT.to_vec()]),
            input: Vec::new(),
        }
    }
}

impl NativePtySpawner for RuntimeSelectionCopySmokePtySpawner {
    type Session = RuntimeSelectionCopySmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeSelectionCopySmokePtySession::default())
    }
}

impl NativePtySessionIo for RuntimeSelectionCopySmokePtySession {
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

pub(super) fn runtime_selection_copy_smoke_exit<C: HostClipboard>(clipboard: &mut C) -> CliExit {
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
        Err(error) => return runtime_selection_copy_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&RuntimeSelectionCopySmokePtySpawner) {
        return runtime_selection_copy_smoke_error(error);
    }
    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(pumped_bytes) => pumped_bytes,
        Err(error) => return runtime_selection_copy_smoke_error(error),
    };

    runtime.set_selection(SelectionRange::new((0, 0), (1, 5)));
    let copied = runtime.copy_selection_to_clipboard(clipboard);
    let metrics = runtime.dump_runtime_perf_metrics();
    let clipboard_text = clipboard.read_text();
    let expected_clipboard = Some(RUNTIME_SELECTION_COPY_SMOKE_TEXT);

    if pumped_bytes != RUNTIME_SELECTION_COPY_SMOKE_OUTPUT.len()
        || !copied
        || clipboard_text.as_deref() != expected_clipboard
        || metrics.pty_input_writes != 0
        || runtime
            .shell_session()
            .is_some_and(|session| !session.input.is_empty())
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr:
                "runtime selection copy smoke failed: selection text did not reach the clipboard\n"
                    .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime selection copy smoke: ok\npumped bytes: {}\ncopied text bytes: {}\nclipboard updated: {}\npty input writes: {}\n",
            pumped_bytes,
            RUNTIME_SELECTION_COPY_SMOKE_TEXT.len(),
            copied,
            metrics.pty_input_writes
        ),
        stderr: String::new(),
    }
}

fn runtime_selection_copy_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime selection copy smoke failed: {error}\n"),
    }
}
