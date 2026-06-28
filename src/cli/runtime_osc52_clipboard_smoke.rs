//! Runtime OSC 52 clipboard CLI smoke command.

use std::collections::VecDeque;

use base64::{Engine as _, engine::general_purpose};

use super::CliExit;
use super::clipboard_smoke::restore_clipboard_after_smoke;
use crate::app::{
    NativePtyResize, NativePtySessionIo, NativePtySpawner, NativeTerminalRuntime,
    NativeTerminalRuntimeConfig,
};
use crate::clipboard::HostClipboard;
use crate::pty::{PtyConfig, PtyError, ShellCommand};

const RUNTIME_OSC52_CLIPBOARD_SMOKE_TEXT: &str = "gromaq runtime osc52 smoke";

#[derive(Debug, Clone, Copy, Default)]
struct RuntimeOsc52ClipboardSmokePtySpawner;

#[derive(Debug)]
struct RuntimeOsc52ClipboardSmokePtySession {
    output: VecDeque<Vec<u8>>,
    input: Vec<Vec<u8>>,
}

impl Default for RuntimeOsc52ClipboardSmokePtySession {
    fn default() -> Self {
        Self {
            output: VecDeque::from([runtime_osc52_clipboard_sequence()]),
            input: Vec::new(),
        }
    }
}

impl NativePtySpawner for RuntimeOsc52ClipboardSmokePtySpawner {
    type Session = RuntimeOsc52ClipboardSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeOsc52ClipboardSmokePtySession::default())
    }
}

impl NativePtySessionIo for RuntimeOsc52ClipboardSmokePtySession {
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

pub(super) fn runtime_osc52_clipboard_smoke_exit<C: HostClipboard>(clipboard: &mut C) -> CliExit {
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
        Err(error) => return runtime_osc52_clipboard_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&RuntimeOsc52ClipboardSmokePtySpawner) {
        return runtime_osc52_clipboard_smoke_error(error);
    }

    let previous_text = clipboard.read_text();
    let expected_output_bytes = runtime_osc52_clipboard_sequence().len();
    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(pumped_bytes) => pumped_bytes,
        Err(error) => return runtime_osc52_clipboard_smoke_error(error),
    };
    let synced = runtime.sync_terminal_clipboard(clipboard);
    let observed = clipboard.read_text();
    let restored_previous_text =
        restore_clipboard_after_smoke(clipboard, previous_text, RUNTIME_OSC52_CLIPBOARD_SMOKE_TEXT);
    let repeat_sync_suppressed = !runtime.sync_terminal_clipboard(clipboard);
    let metrics = runtime.dump_runtime_perf_metrics();

    if pumped_bytes != expected_output_bytes
        || !synced
        || !repeat_sync_suppressed
        || observed.as_deref() != Some(RUNTIME_OSC52_CLIPBOARD_SMOKE_TEXT)
        || metrics.pty_input_writes != 0
        || runtime
            .shell_session()
            .is_some_and(|session| !session.input.is_empty())
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr:
                "runtime OSC 52 clipboard smoke failed: terminal clipboard did not sync to host\n"
                    .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime OSC 52 clipboard smoke: ok\npumped bytes: {}\ndecoded bytes: {}\nclipboard synced: {}\nrepeat sync suppressed: {}\npty input writes: {}\nprevious text restored: {}\n",
            pumped_bytes,
            RUNTIME_OSC52_CLIPBOARD_SMOKE_TEXT.len(),
            synced,
            repeat_sync_suppressed,
            metrics.pty_input_writes,
            restored_previous_text
        ),
        stderr: String::new(),
    }
}

fn runtime_osc52_clipboard_sequence() -> Vec<u8> {
    let payload = general_purpose::STANDARD.encode(RUNTIME_OSC52_CLIPBOARD_SMOKE_TEXT);
    format!("\x1b]52;c;{payload}\x07").into_bytes()
}

fn runtime_osc52_clipboard_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime OSC 52 clipboard smoke failed: {error}\n"),
    }
}
