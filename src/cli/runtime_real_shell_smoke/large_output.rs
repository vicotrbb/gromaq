use std::thread;
use std::time::Instant;

use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, RealNativePtySpawner};
use crate::cli::CliExit;
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::support::{
    real_shell_large_output_command, real_shell_large_output_line, runtime_transcript,
};
use super::{
    REAL_SHELL_LARGE_OUTPUT_LINES, REAL_SHELL_LARGE_OUTPUT_SCROLLBACK_LINES, REAL_SHELL_SMOKE_COLS,
    REAL_SHELL_SMOKE_POLL_INTERVAL, REAL_SHELL_SMOKE_ROWS, REAL_SHELL_SMOKE_TIMEOUT,
};

mod budget;
mod output;

use budget::{
    REAL_SHELL_LARGE_OUTPUT_RENDER_P95_BUDGET_NS, real_shell_large_output_budget_failure,
};
use output::{
    RuntimeRealShellLargeOutputReport, runtime_real_shell_large_output_smoke_error,
    runtime_real_shell_large_output_smoke_failure, runtime_real_shell_large_output_smoke_success,
};

pub(in crate::cli) fn runtime_real_shell_large_output_smoke_exit() -> CliExit {
    let probe = match run_runtime_real_shell_large_output_smoke() {
        Ok(probe) => probe,
        Err(error) => return runtime_real_shell_large_output_smoke_error(error),
    };
    if let Some(failure) = real_shell_large_output_budget_failure(&probe) {
        return runtime_real_shell_large_output_smoke_failure(failure);
    }

    runtime_real_shell_large_output_smoke_success(&RuntimeRealShellLargeOutputReport {
        lines: REAL_SHELL_LARGE_OUTPUT_LINES,
        pumped_bytes: probe.pumped_bytes,
        scrollback_cap: REAL_SHELL_LARGE_OUTPUT_SCROLLBACK_LINES,
        scrollback_lines: probe.scrollback_lines,
        rendered_frames: probe.rendered_frames,
        rendered_dirty_regions: probe.rendered_dirty_regions,
        rendered_dirty_cells_max: probe.rendered_dirty_cells_max,
        first_line_evicted: probe.first_line_evicted,
        last_line_observed: probe.last_line_observed,
        render_p95_ns: probe.render_p95_ns,
        render_p95_budget_ns: REAL_SHELL_LARGE_OUTPUT_RENDER_P95_BUDGET_NS,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RuntimeRealShellLargeOutputSmokeProbe {
    pumped_bytes: usize,
    scrollback_lines: usize,
    rendered_frames: u64,
    rendered_dirty_regions: u64,
    rendered_dirty_cells_max: u64,
    first_line_evicted: bool,
    last_line_observed: bool,
    render_p95_ns: u64,
}

type RuntimeRealShellLargeOutputResult = Result<RuntimeRealShellLargeOutputSmokeProbe, String>;

fn run_runtime_real_shell_large_output_smoke() -> RuntimeRealShellLargeOutputResult {
    let first_line = real_shell_large_output_line(0);
    let last_line = real_shell_large_output_line(REAL_SHELL_LARGE_OUTPUT_LINES - 1);
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: REAL_SHELL_SMOKE_COLS,
        terminal_rows: REAL_SHELL_SMOKE_ROWS,
        scrollback_lines: REAL_SHELL_LARGE_OUTPUT_SCROLLBACK_LINES,
        pixel_width: 0,
        pixel_height: 0,
        cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
        cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
        shell: real_shell_large_output_command(),
    })
    .map_err(|error| error.to_string())?;
    runtime
        .start_shell(&RealNativePtySpawner::default())
        .map_err(|error| error.to_string())?;

    let mut renderer =
        WgpuRenderer::new(RendererConfig::default()).map_err(|error| error.to_string())?;
    let mut pumped_bytes = 0;
    let deadline = Instant::now() + REAL_SHELL_SMOKE_TIMEOUT;
    loop {
        let pumped = runtime
            .pump_pty_output()
            .map_err(|error| error.to_string())?;
        if pumped > 0 {
            pumped_bytes += pumped;
            runtime
                .render_terminal_frame(&mut renderer)
                .map_err(|error| error.to_string())?;
        }

        let transcript = runtime_transcript(&runtime);
        let scrollback = runtime.terminal().dump_scrollback();
        let first_line_evicted = !scrollback.lines.iter().any(|line| line == &first_line);
        let last_line_observed = transcript.contains(&last_line);
        if pumped_bytes > 0
            && scrollback.lines.len() == REAL_SHELL_LARGE_OUTPUT_SCROLLBACK_LINES
            && first_line_evicted
            && last_line_observed
        {
            let metrics = runtime.dump_runtime_perf_metrics();
            if metrics.rendered_frames == 0
                || metrics.rendered_dirty_regions == 0
                || metrics.rendered_dirty_cells_max == 0
            {
                return Err("real shell large output did not render dirty frames".to_owned());
            }
            return Ok(RuntimeRealShellLargeOutputSmokeProbe {
                pumped_bytes,
                scrollback_lines: scrollback.lines.len(),
                rendered_frames: metrics.rendered_frames,
                rendered_dirty_regions: metrics.rendered_dirty_regions,
                rendered_dirty_cells_max: metrics.rendered_dirty_cells_max,
                first_line_evicted,
                last_line_observed,
                render_p95_ns: metrics.render_time_p95_ns,
            });
        }

        if Instant::now() >= deadline {
            return Err(format!(
                "timed out waiting for real shell large output; observed: {}",
                transcript.replace('\n', "|")
            ));
        }
        thread::sleep(REAL_SHELL_SMOKE_POLL_INTERVAL);
    }
}
