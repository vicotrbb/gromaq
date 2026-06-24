//! Clipboard CLI smoke commands.

use base64::{Engine as _, engine::general_purpose};

use super::CliExit;
use crate::clipboard::HostClipboard;
use crate::terminal::{Terminal, TerminalConfig};

const CLIPBOARD_SMOKE_TEXT: &str = "gromaq clipboard smoke";
const OSC52_CLIPBOARD_SMOKE_TEXT: &str = "gromaq osc52 smoke";

pub(super) fn clipboard_smoke_exit<C: HostClipboard>(clipboard: &mut C) -> CliExit {
    let previous_text = clipboard.read_text();
    clipboard.write_text(CLIPBOARD_SMOKE_TEXT);
    let observed = clipboard.read_text();
    let restored_previous_text =
        restore_clipboard_after_smoke(clipboard, previous_text, CLIPBOARD_SMOKE_TEXT);

    match observed {
        Some(text) if text == CLIPBOARD_SMOKE_TEXT => CliExit {
            code: 0,
            stdout: format!(
                "clipboard smoke: ok\nroundtrip bytes: {}\nprevious text restored: {}\n",
                CLIPBOARD_SMOKE_TEXT.len(),
                restored_previous_text
            ),
            stderr: String::new(),
        },
        Some(text) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!(
                "clipboard smoke failed: expected {CLIPBOARD_SMOKE_TEXT:?}, read {text:?}\n"
            ),
        },
        None => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "clipboard smoke failed: read no text after write\n".to_owned(),
        },
    }
}

pub(super) fn osc52_clipboard_smoke_exit<C: HostClipboard>(clipboard: &mut C) -> CliExit {
    let previous_text = clipboard.read_text();
    let config = match TerminalConfig::new(24, 3) {
        Ok(config) => config,
        Err(error) => {
            return CliExit {
                code: 1,
                stdout: String::new(),
                stderr: format!("OSC 52 clipboard smoke failed: {error}\n"),
            };
        }
    };
    let mut terminal = Terminal::new(config);
    let payload = general_purpose::STANDARD.encode(OSC52_CLIPBOARD_SMOKE_TEXT);
    let sequence = format!("\x1b]52;c;{payload}\x07");
    if let Err(error) = terminal.write_str(&sequence) {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("OSC 52 clipboard smoke failed: {error}\n"),
        };
    }
    let Some(decoded_text) = terminal.dump_clipboard_text() else {
        restore_clipboard_after_smoke(clipboard, previous_text, OSC52_CLIPBOARD_SMOKE_TEXT);
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "OSC 52 clipboard smoke failed: terminal decoded no clipboard text\n"
                .to_owned(),
        };
    };
    if decoded_text != OSC52_CLIPBOARD_SMOKE_TEXT {
        restore_clipboard_after_smoke(clipboard, previous_text, OSC52_CLIPBOARD_SMOKE_TEXT);
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!(
                "OSC 52 clipboard smoke failed: expected decoded text {OSC52_CLIPBOARD_SMOKE_TEXT:?}, got {decoded_text:?}\n"
            ),
        };
    }

    clipboard.write_text(&decoded_text);
    let observed = clipboard.read_text();
    let restored_previous_text =
        restore_clipboard_after_smoke(clipboard, previous_text, OSC52_CLIPBOARD_SMOKE_TEXT);

    match observed {
        Some(text) if text == OSC52_CLIPBOARD_SMOKE_TEXT => CliExit {
            code: 0,
            stdout: format!(
                "OSC 52 clipboard smoke: ok\ndecoded bytes: {}\nprevious text restored: {}\n",
                OSC52_CLIPBOARD_SMOKE_TEXT.len(),
                restored_previous_text
            ),
            stderr: String::new(),
        },
        Some(text) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!(
                "OSC 52 clipboard smoke failed: expected clipboard text {OSC52_CLIPBOARD_SMOKE_TEXT:?}, read {text:?}\n"
            ),
        },
        None => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "OSC 52 clipboard smoke failed: read no text after write\n".to_owned(),
        },
    }
}

pub(super) fn restore_clipboard_after_smoke<C: HostClipboard>(
    clipboard: &mut C,
    previous_text: Option<String>,
    sentinel_text: &str,
) -> bool {
    let restorable_previous_text = previous_text
        .as_deref()
        .filter(|text| *text != sentinel_text);
    let restored_previous_text = restorable_previous_text.is_some();
    match restorable_previous_text {
        Some(previous_text) => clipboard.write_text(previous_text),
        None => clipboard.write_text(""),
    }
    restored_previous_text
}
