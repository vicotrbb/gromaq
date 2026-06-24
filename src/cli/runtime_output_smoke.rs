//! Runtime output CLI smoke commands.

pub(super) use bounded_state::runtime_bounded_state_smoke_exit;
pub(super) use continuous_output::runtime_continuous_output_smoke_exit;
pub(super) use large_output::runtime_large_output_smoke_exit;

mod bounded_state;
mod continuous_output;
mod large_output;
mod pty_smoke;

const RUNTIME_OUTPUT_SMOKE_COLS: u16 = 32;
const RUNTIME_OUTPUT_SMOKE_ROWS: u16 = 8;
const RUNTIME_LARGE_OUTPUT_LINES: usize = 512;
const RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES: usize = 128;
const RUNTIME_BOUNDED_STATE_BATCHES: usize = 4;
const RUNTIME_CONTINUOUS_OUTPUT_BATCHES: usize = 32;
const RUNTIME_CONTINUOUS_OUTPUT_LINES_PER_BATCH: usize = 8;
const RUNTIME_CONTINUOUS_OUTPUT_SCROLLBACK_LINES: usize = 64;

fn runtime_output_smoke_viewport_cells() -> u64 {
    u64::from(RUNTIME_OUTPUT_SMOKE_COLS) * u64::from(RUNTIME_OUTPUT_SMOKE_ROWS)
}
