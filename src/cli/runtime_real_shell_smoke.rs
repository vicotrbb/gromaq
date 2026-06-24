//! Real shell runtime CLI smoke commands.

pub(super) use interactive::runtime_real_shell_smoke_exit;
pub(super) use large_output::runtime_real_shell_large_output_smoke_exit;
pub(super) use reflow::runtime_real_shell_reflow_smoke_exit;

mod interactive;
mod large_output;
mod reflow;
mod support;

use std::time::Duration;

const REAL_SHELL_SMOKE_COLS: u16 = 48;
const REAL_SHELL_SMOKE_ROWS: u16 = 8;
const REAL_SHELL_SMOKE_TIMEOUT: Duration = Duration::from_secs(3);
const REAL_SHELL_SMOKE_POLL_INTERVAL: Duration = Duration::from_millis(10);
const REAL_SHELL_READY: &str = "gromaq-real-shell-ready";
const REAL_SHELL_INPUT: &str = "gromaq-real-shell-input";
const REAL_SHELL_EXIT: &str = "gromaq-real-shell-exit";
const REAL_SHELL_LARGE_OUTPUT_LINES: usize = 256;
const REAL_SHELL_LARGE_OUTPUT_SCROLLBACK_LINES: usize = 64;
const REAL_SHELL_REFLOW_OUTPUT: &str = "abcdefghij\nklmnopqrst\nuv";
