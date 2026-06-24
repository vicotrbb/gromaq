use std::ffi::OsString;

use crate::app::NativeTerminalRuntime;
use crate::pty::ShellCommand;

use super::{
    REAL_SHELL_EXIT, REAL_SHELL_LARGE_OUTPUT_LINES, REAL_SHELL_READY, REAL_SHELL_REFLOW_OUTPUT,
};

pub(super) fn real_shell_command() -> ShellCommand {
    ShellCommand {
        program: OsString::from("/bin/sh"),
        args: vec![
            OsString::from("-c"),
            OsString::from(format!(
                "printf '{REAL_SHELL_READY}\\n'; \
                 while IFS= read -r line; do \
                 printf 'gromaq-real-shell-echo:%s\\n' \"$line\"; \
                 [ \"$line\" = '{REAL_SHELL_EXIT}' ] && exit 0; \
                 done"
            )),
        ],
        cwd: None,
    }
}

pub(super) fn real_shell_large_output_command() -> ShellCommand {
    ShellCommand {
        program: OsString::from("/bin/sh"),
        args: vec![
            OsString::from("-c"),
            OsString::from(format!(
                "i=0; \
                 while [ \"$i\" -lt {REAL_SHELL_LARGE_OUTPUT_LINES} ]; do \
                 printf 'gromaq-real-large-line-%03d\\n' \"$i\"; \
                 i=$((i + 1)); \
                 done"
            )),
        ],
        cwd: None,
    }
}

pub(super) fn real_shell_reflow_command() -> ShellCommand {
    ShellCommand {
        program: OsString::from("/bin/sh"),
        args: vec![
            OsString::from("-c"),
            OsString::from(format!("printf '{}'", REAL_SHELL_REFLOW_OUTPUT)),
        ],
        cwd: None,
    }
}

pub(super) fn real_shell_large_output_line(line: usize) -> String {
    format!("gromaq-real-large-line-{line:03}")
}

pub(super) fn runtime_transcript<S>(runtime: &NativeTerminalRuntime<S>) -> String {
    let mut transcript = runtime.terminal().dump_scrollback().lines.join("\n");
    let grid = runtime.terminal().dump_grid();
    for row in 0..grid.rows {
        if !transcript.is_empty() {
            transcript.push('\n');
        }
        transcript.push_str(&grid.line_text(row));
    }
    transcript
}
