use std::collections::VecDeque;

use crate::app::{
    NativePtyResize, NativePtySessionIo, NativePtySpawner, NativeTerminalRuntime,
    NativeTerminalRuntimeConfig,
};
use crate::pty::{PtyConfig, PtyError, ShellCommand};
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::CliExit;

const RUNTIME_OUTPUT_SMOKE_COLS: u16 = 32;
const RUNTIME_OUTPUT_SMOKE_ROWS: u16 = 8;
const RUNTIME_LARGE_OUTPUT_LINES: usize = 512;
const RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES: usize = 128;
const RUNTIME_BOUNDED_STATE_BATCHES: usize = 4;
const RUNTIME_CONTINUOUS_OUTPUT_BATCHES: usize = 32;
const RUNTIME_CONTINUOUS_OUTPUT_LINES_PER_BATCH: usize = 8;
const RUNTIME_CONTINUOUS_OUTPUT_SCROLLBACK_LINES: usize = 64;

#[derive(Debug, Clone)]
struct RuntimeLargeOutputSmokePtySpawner {
    payload: Vec<u8>,
}

#[derive(Debug)]
struct RuntimeLargeOutputSmokePtySession {
    output: VecDeque<Vec<u8>>,
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

fn runtime_large_output_payload(lines: usize) -> Vec<u8> {
    let mut payload = Vec::new();
    for line in 0..lines {
        payload.extend_from_slice(format!("gromaq-runtime-line-{line:03}\n").as_bytes());
    }
    payload
}

pub(super) fn runtime_large_output_smoke_exit() -> CliExit {
    let payload = runtime_large_output_payload(RUNTIME_LARGE_OUTPUT_LINES);
    let expected_bytes = payload.len();
    let last_line = format!("gromaq-runtime-line-{:03}", RUNTIME_LARGE_OUTPUT_LINES - 1);
    let viewport_cells = runtime_output_smoke_viewport_cells();
    let spawner = RuntimeLargeOutputSmokePtySpawner { payload };
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: RUNTIME_OUTPUT_SMOKE_COLS,
        terminal_rows: RUNTIME_OUTPUT_SMOKE_ROWS,
        scrollback_lines: RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_large_output_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_large_output_smoke_error(error);
    }

    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_large_output_smoke_error(error),
    };
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_large_output_smoke_error(error),
    };
    let rendered = match runtime.render_terminal_frame(&mut renderer) {
        Ok(rendered) => rendered,
        Err(error) => return runtime_large_output_smoke_error(error),
    };
    let metrics = runtime.dump_runtime_perf_metrics();
    let scrollback = runtime.terminal().dump_scrollback();
    let visible_text = renderer
        .last_plan()
        .map(|plan| {
            plan.glyphs
                .iter()
                .map(|glyph| glyph.text.as_str())
                .collect::<String>()
        })
        .unwrap_or_default();

    if pumped_bytes != expected_bytes
        || metrics.pty_output_bytes != expected_bytes as u64
        || metrics.pty_output_batches != 1
        || !rendered
        || metrics.rendered_frames != 1
        || metrics.rendered_dirty_regions == 0
        || metrics.rendered_dirty_cells == 0
        || metrics.rendered_dirty_cells_max == 0
        || metrics.rendered_dirty_cells_max > viewport_cells
        || scrollback.lines.len() != RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES
        || scrollback
            .lines
            .iter()
            .any(|line| line == "gromaq-runtime-line-000")
        || !visible_text.contains(&last_line)
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr:
                "runtime large-output smoke failed: burst did not reach a rendered visible frame\n"
                    .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime large-output smoke: ok\nlines: {}\npumped bytes: {}\nscrollback lines: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells: {}\nrendered dirty cells max: {}\nlast visible line: {}\nrender p95 ns: {}\n",
            RUNTIME_LARGE_OUTPUT_LINES,
            pumped_bytes,
            scrollback.lines.len(),
            metrics.rendered_frames,
            metrics.rendered_dirty_regions,
            metrics.rendered_dirty_cells,
            metrics.rendered_dirty_cells_max,
            last_line,
            metrics.render_time_p95_ns
        ),
        stderr: String::new(),
    }
}

fn runtime_output_smoke_viewport_cells() -> u64 {
    u64::from(RUNTIME_OUTPUT_SMOKE_COLS) * u64::from(RUNTIME_OUTPUT_SMOKE_ROWS)
}

fn runtime_large_output_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime large-output smoke failed: {error}\n"),
    }
}

#[derive(Debug, Clone)]
struct RuntimeChunkedOutputSmokePtySpawner {
    payloads: Vec<Vec<u8>>,
}

#[derive(Debug)]
struct RuntimeChunkedOutputSmokePtySession {
    output: VecDeque<Vec<u8>>,
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

fn runtime_bounded_state_payloads() -> Vec<Vec<u8>> {
    (0..RUNTIME_BOUNDED_STATE_BATCHES)
        .map(|batch| {
            let start = batch * RUNTIME_LARGE_OUTPUT_LINES;
            let end = start + RUNTIME_LARGE_OUTPUT_LINES;
            let mut payload = Vec::new();
            for line in start..end {
                payload.extend_from_slice(format!("gromaq-bounded-line-{line:04}\n").as_bytes());
            }
            payload
        })
        .collect()
}

pub(super) fn runtime_bounded_state_smoke_exit() -> CliExit {
    let payloads = runtime_bounded_state_payloads();
    let expected_bytes: usize = payloads.iter().map(Vec::len).sum();
    let total_lines = RUNTIME_LARGE_OUTPUT_LINES * RUNTIME_BOUNDED_STATE_BATCHES;
    let last_line = format!("gromaq-bounded-line-{:04}", total_lines - 1);
    let viewport_cells = runtime_output_smoke_viewport_cells();
    let spawner = RuntimeChunkedOutputSmokePtySpawner { payloads };
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: RUNTIME_OUTPUT_SMOKE_COLS,
        terminal_rows: RUNTIME_OUTPUT_SMOKE_ROWS,
        scrollback_lines: RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_bounded_state_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_bounded_state_smoke_error(error);
    }

    let mut pumped_bytes = 0_usize;
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_bounded_state_smoke_error(error),
    };
    for _ in 0..RUNTIME_BOUNDED_STATE_BATCHES {
        let batch_bytes = match runtime.pump_pty_output() {
            Ok(bytes) => bytes,
            Err(error) => return runtime_bounded_state_smoke_error(error),
        };
        pumped_bytes = pumped_bytes.saturating_add(batch_bytes);
        let rendered = match runtime.render_terminal_frame(&mut renderer) {
            Ok(rendered) => rendered,
            Err(error) => return runtime_bounded_state_smoke_error(error),
        };
        if batch_bytes == 0 || !rendered {
            return runtime_bounded_state_smoke_failure(
                "output batch did not render a dirty frame",
            );
        }
        let state = runtime.dump_runtime_state_snapshot();
        if state.scrollback_lines > RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES
            || state.scrollback_cell_rows > RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES
            || state.scrollback_cells > state.scrollback_cell_limit
        {
            return runtime_bounded_state_smoke_failure("scrollback state exceeded configured cap");
        }
    }

    let metrics = runtime.dump_runtime_perf_metrics();
    let state = runtime.dump_runtime_state_snapshot();
    let scrollback = runtime.terminal().dump_scrollback();
    let visible_text = renderer
        .last_plan()
        .map(|plan| {
            plan.glyphs
                .iter()
                .map(|glyph| glyph.text.as_str())
                .collect::<String>()
        })
        .unwrap_or_default();

    if pumped_bytes != expected_bytes
        || metrics.pty_output_batches != RUNTIME_BOUNDED_STATE_BATCHES as u64
        || metrics.pty_output_bytes != expected_bytes as u64
        || metrics.rendered_frames != RUNTIME_BOUNDED_STATE_BATCHES as u64
        || metrics.rendered_dirty_regions == 0
        || metrics.rendered_dirty_cells == 0
        || metrics.rendered_dirty_cells_max == 0
        || metrics.rendered_dirty_cells_max > viewport_cells
        || state.scrollback_limit != RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES
        || state.scrollback_lines != RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES
        || state.scrollback_cell_rows != RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES
        || state.scrollback_cells > state.scrollback_cell_limit
        || scrollback
            .lines
            .iter()
            .any(|line| line == "gromaq-bounded-line-0000")
        || !visible_text.contains(&last_line)
    {
        return runtime_bounded_state_smoke_failure(
            "long-session output did not stay bounded while preserving latest visible content",
        );
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime bounded-state smoke: ok\nbatches: {}\nlines: {}\npumped bytes: {}\nscrollback cap: {}\nscrollback lines: {}\nscrollback cell rows: {}\nscrollback cells: {}\nscrollback max cells: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells: {}\nrendered dirty cells max: {}\nlast visible line: {}\n",
            RUNTIME_BOUNDED_STATE_BATCHES,
            total_lines,
            pumped_bytes,
            state.scrollback_limit,
            state.scrollback_lines,
            state.scrollback_cell_rows,
            state.scrollback_cells,
            state.scrollback_cell_limit,
            metrics.rendered_frames,
            metrics.rendered_dirty_regions,
            metrics.rendered_dirty_cells,
            metrics.rendered_dirty_cells_max,
            last_line
        ),
        stderr: String::new(),
    }
}

fn runtime_bounded_state_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime bounded-state smoke failed: {error}\n"),
    }
}

fn runtime_bounded_state_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime bounded-state smoke failed: {reason}\n"),
    }
}

fn runtime_continuous_output_payloads() -> Vec<Vec<u8>> {
    (0..RUNTIME_CONTINUOUS_OUTPUT_BATCHES)
        .map(|batch| {
            let start = batch * RUNTIME_CONTINUOUS_OUTPUT_LINES_PER_BATCH;
            let end = start + RUNTIME_CONTINUOUS_OUTPUT_LINES_PER_BATCH;
            let mut payload = Vec::new();
            for line in start..end {
                payload.extend_from_slice(format!("gromaq-continuous-line-{line:03}\n").as_bytes());
            }
            payload
        })
        .collect()
}

pub(super) fn runtime_continuous_output_smoke_exit() -> CliExit {
    let payloads = runtime_continuous_output_payloads();
    let expected_bytes: usize = payloads.iter().map(Vec::len).sum();
    let total_lines = RUNTIME_CONTINUOUS_OUTPUT_BATCHES * RUNTIME_CONTINUOUS_OUTPUT_LINES_PER_BATCH;
    let last_line = format!("gromaq-continuous-line-{:03}", total_lines - 1);
    let viewport_cells = runtime_output_smoke_viewport_cells();
    let spawner = RuntimeChunkedOutputSmokePtySpawner { payloads };
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: RUNTIME_OUTPUT_SMOKE_COLS,
        terminal_rows: RUNTIME_OUTPUT_SMOKE_ROWS,
        scrollback_lines: RUNTIME_CONTINUOUS_OUTPUT_SCROLLBACK_LINES,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_continuous_output_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_continuous_output_smoke_error(error);
    }

    let mut pumped_bytes = 0_usize;
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_continuous_output_smoke_error(error),
    };
    for _ in 0..RUNTIME_CONTINUOUS_OUTPUT_BATCHES {
        let batch_bytes = match runtime.pump_pty_output() {
            Ok(bytes) => bytes,
            Err(error) => return runtime_continuous_output_smoke_error(error),
        };
        pumped_bytes = pumped_bytes.saturating_add(batch_bytes);
        let rendered = match runtime.render_terminal_frame(&mut renderer) {
            Ok(rendered) => rendered,
            Err(error) => return runtime_continuous_output_smoke_error(error),
        };
        if batch_bytes == 0 || !rendered {
            return runtime_continuous_output_smoke_failure(
                "stream batch did not render a dirty frame",
            );
        }
    }

    let metrics = runtime.dump_runtime_perf_metrics();
    let scrollback = runtime.terminal().dump_scrollback();
    let visible_text = renderer
        .last_plan()
        .map(|plan| {
            plan.glyphs
                .iter()
                .map(|glyph| glyph.text.as_str())
                .collect::<String>()
        })
        .unwrap_or_default();

    if pumped_bytes != expected_bytes
        || metrics.pty_output_batches != RUNTIME_CONTINUOUS_OUTPUT_BATCHES as u64
        || metrics.pty_output_bytes != expected_bytes as u64
        || metrics.rendered_frames != RUNTIME_CONTINUOUS_OUTPUT_BATCHES as u64
        || metrics.rendered_dirty_regions == 0
        || metrics.rendered_dirty_cells == 0
        || metrics.rendered_dirty_cells_max == 0
        || metrics.rendered_dirty_cells_max > viewport_cells
        || scrollback.lines.len() != RUNTIME_CONTINUOUS_OUTPUT_SCROLLBACK_LINES
        || scrollback
            .lines
            .iter()
            .any(|line| line == "gromaq-continuous-line-000")
        || !visible_text.contains(&last_line)
    {
        return runtime_continuous_output_smoke_failure(
            "continuous output stream did not stay responsive and bounded",
        );
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime continuous-output smoke: ok\nbatches: {}\nlines: {}\npumped bytes: {}\nscrollback lines: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells: {}\nrendered dirty cells max: {}\nlast visible line: {}\nrender p95 ns: {}\n",
            RUNTIME_CONTINUOUS_OUTPUT_BATCHES,
            total_lines,
            pumped_bytes,
            scrollback.lines.len(),
            metrics.rendered_frames,
            metrics.rendered_dirty_regions,
            metrics.rendered_dirty_cells,
            metrics.rendered_dirty_cells_max,
            last_line,
            metrics.render_time_p95_ns
        ),
        stderr: String::new(),
    }
}

fn runtime_continuous_output_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime continuous-output smoke failed: {error}\n"),
    }
}

fn runtime_continuous_output_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime continuous-output smoke failed: {reason}\n"),
    }
}
