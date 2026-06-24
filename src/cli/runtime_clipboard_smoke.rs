//! Runtime clipboard-paste CLI smoke command.

use winit::keyboard::{Key, ModifiersState, NamedKey};

use super::CliExit;
use super::clipboard_smoke::restore_clipboard_after_smoke;
use crate::app::{
    NativePtyResize, NativePtySessionIo, NativePtySpawner, NativeTerminalRuntime,
    NativeTerminalRuntimeConfig, is_native_paste_shortcut,
};
use crate::clipboard::HostClipboard;
use crate::pty::{PtyConfig, PtyError, ShellCommand};

const RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT: &str = "gromaq runtime clipboard paste";

#[derive(Debug, Clone, Copy, Default)]
struct RuntimeClipboardPasteSmokePtySpawner;

#[derive(Debug, Default)]
struct RuntimeClipboardPasteSmokePtySession {
    input: Vec<Vec<u8>>,
}

impl NativePtySpawner for RuntimeClipboardPasteSmokePtySpawner {
    type Session = RuntimeClipboardPasteSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeClipboardPasteSmokePtySession::default())
    }
}

impl NativePtySessionIo for RuntimeClipboardPasteSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(Vec::new())
    }

    fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.input.push(bytes.to_vec());
        Ok(())
    }

    fn resize(&mut self, _size: NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}

pub(super) fn runtime_clipboard_paste_smoke_exit<C: HostClipboard>(clipboard: &mut C) -> CliExit {
    let paste_key_recognized =
        is_native_paste_shortcut(&Key::Named(NamedKey::Paste), ModifiersState::empty());
    if !paste_key_recognized {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "runtime clipboard paste smoke failed: OS Paste key was not recognized\n"
                .to_owned(),
        };
    }
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 24,
        terminal_rows: 4,
        scrollback_lines: 128,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_clipboard_paste_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&RuntimeClipboardPasteSmokePtySpawner) {
        return runtime_clipboard_paste_smoke_error(error);
    }

    let previous_text = clipboard.read_text();
    clipboard.write_text(RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT);
    let paste_result = runtime.send_clipboard_paste(clipboard);
    let restored_previous_text =
        restore_clipboard_after_smoke(clipboard, previous_text, RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT);
    let pasted = match paste_result {
        Ok(pasted) => pasted,
        Err(error) => return runtime_clipboard_paste_smoke_error(error),
    };
    let metrics = runtime.dump_runtime_perf_metrics();
    let pasted_bytes = runtime
        .shell_session()
        .and_then(|session| session.input.last())
        .map(Vec::as_slice);

    if !pasted
        || pasted_bytes != Some(RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT.as_bytes())
        || metrics.clipboard_pastes != 1
        || metrics.paste_bytes != RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT.len() as u64
        || metrics.pty_input_writes != 1
        || metrics.pty_input_bytes != RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT.len() as u64
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "runtime clipboard paste smoke failed: clipboard text did not reach the PTY\n"
                .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime clipboard paste smoke: ok\npaste key recognized: {}\npasted bytes: {}\nclipboard pastes: {}\nprevious text restored: {}\n",
            paste_key_recognized,
            RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT.len(),
            metrics.clipboard_pastes,
            restored_previous_text
        ),
        stderr: String::new(),
    }
}

fn runtime_clipboard_paste_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime clipboard paste smoke failed: {error}\n"),
    }
}
