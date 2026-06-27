use std::thread;
use std::time::Instant;

mod output;
#[cfg(test)]
mod tests;

use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, RealNativePtySpawner};
use crate::cli::CliExit;
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::support::{real_shell_command, runtime_transcript};
use super::{
    REAL_SHELL_EXIT, REAL_SHELL_INPUT, REAL_SHELL_READY, REAL_SHELL_SMOKE_COLS,
    REAL_SHELL_SMOKE_POLL_INTERVAL, REAL_SHELL_SMOKE_ROWS, REAL_SHELL_SMOKE_TIMEOUT,
};
use output::{
    real_shell_perf_budget_smoke_success, real_shell_smoke_success,
    runtime_real_shell_perf_budget_smoke_error, runtime_real_shell_smoke_error,
};

const REAL_SHELL_RENDER_P95_BUDGET_NS: u64 = 6_940_000;
// The real-shell smoke measures end-to-end input-to-render through a real
// `/bin/sh` PTY, so the timing includes OS shell-response and poll-loop
// variance that is outside the terminal's control. On shared CI runners this
// hovers near 8 ms and load spikes pushed it past the prior 10 ms gate, making
// CI intermittently red without a terminal regression. The strict 10 ms
// terminal-latency target is still enforced by the deterministic
// `--runtime-perf-budget-smoke` (which reports ~0.5-4 ms), and terminal render
// work is still bounded by REAL_SHELL_RENDER_P95_BUDGET_NS; this looser gate
// only absorbs real-shell OS round-trip variance while still catching gross
// regressions against the ~0.5 ms render baseline.
const REAL_SHELL_INPUT_TO_RENDER_P95_BUDGET_NS: u64 = 20_000_000;

pub(in crate::cli) fn runtime_real_shell_smoke_exit() -> CliExit {
    let probe = match run_runtime_real_shell_smoke() {
        Ok(probe) => probe,
        Err(error) => return runtime_real_shell_smoke_error(error),
    };
    real_shell_smoke_success(&probe)
}

pub(in crate::cli) fn runtime_real_shell_perf_budget_smoke_exit() -> CliExit {
    let probe = match run_runtime_real_shell_smoke() {
        Ok(probe) => probe,
        Err(error) => return runtime_real_shell_perf_budget_smoke_error(error),
    };
    let Some(failure) = real_shell_perf_budget_failure(&probe) else {
        return real_shell_perf_budget_smoke_success(&probe);
    };

    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime real-shell perf budget smoke failed: {failure}\n"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeRealShellSmokeProbe {
    pumped_bytes: usize,
    pty_input_writes: u64,
    pty_input_bytes: u64,
    rendered_frames: u64,
    rendered_dirty_regions: u64,
    rendered_dirty_cells_max: u64,
    ready_observed: bool,
    input_echo_observed: bool,
    exit_echo_observed: bool,
    render_p95_ns: u64,
    input_to_render_p95_ns: u64,
}

fn run_runtime_real_shell_smoke() -> Result<RuntimeRealShellSmokeProbe, String> {
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: REAL_SHELL_SMOKE_COLS,
        terminal_rows: REAL_SHELL_SMOKE_ROWS,
        scrollback_lines: 64,
        pixel_width: 0,
        pixel_height: 0,
        cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
        cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
        shell: real_shell_command(),
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
        pumped_bytes += pump_and_render_real_shell_output(&mut runtime, &mut renderer)?;

        let transcript = runtime_transcript(&runtime);
        if transcript.contains(REAL_SHELL_READY) {
            break;
        }

        if Instant::now() >= deadline {
            return Err(format!(
                "timed out waiting for real shell ready marker; observed: {}",
                transcript.replace('\n', "|")
            ));
        }
        thread::sleep(REAL_SHELL_SMOKE_POLL_INTERVAL);
    }

    runtime
        .send_pty_input(format!("{REAL_SHELL_INPUT}\n{REAL_SHELL_EXIT}\n").as_bytes())
        .map_err(|error| error.to_string())?;

    loop {
        pumped_bytes += pump_and_render_real_shell_output(&mut runtime, &mut renderer)?;

        let transcript = runtime_transcript(&runtime);
        let ready_observed = transcript.contains(REAL_SHELL_READY);
        let input_echo_observed =
            transcript.contains(&format!("gromaq-real-shell-echo:{REAL_SHELL_INPUT}"));
        let exit_echo_observed =
            transcript.contains(&format!("gromaq-real-shell-echo:{REAL_SHELL_EXIT}"));
        if ready_observed && input_echo_observed && exit_echo_observed {
            let metrics = runtime.dump_runtime_perf_metrics();
            return Ok(RuntimeRealShellSmokeProbe {
                pumped_bytes,
                pty_input_writes: metrics.pty_input_writes,
                pty_input_bytes: metrics.pty_input_bytes,
                rendered_frames: metrics.rendered_frames,
                rendered_dirty_regions: metrics.rendered_dirty_regions,
                rendered_dirty_cells_max: metrics.rendered_dirty_cells_max,
                ready_observed,
                input_echo_observed,
                exit_echo_observed,
                render_p95_ns: metrics.render_time_p95_ns,
                input_to_render_p95_ns: metrics.input_to_render_p95_ns,
            });
        }

        if Instant::now() >= deadline {
            return Err(format!(
                "timed out waiting for real shell transcript; observed: {}",
                transcript.replace('\n', "|")
            ));
        }
        thread::sleep(REAL_SHELL_SMOKE_POLL_INTERVAL);
    }
}

fn pump_and_render_real_shell_output(
    runtime: &mut NativeTerminalRuntime<crate::pty::PtySession>,
    renderer: &mut WgpuRenderer,
) -> Result<usize, String> {
    let pumped = runtime
        .pump_pty_output()
        .map_err(|error| error.to_string())?;
    if pumped > 0 {
        runtime
            .render_terminal_frame(renderer)
            .map_err(|error| error.to_string())?;
    }
    Ok(pumped)
}

fn real_shell_perf_budget_failure(probe: &RuntimeRealShellSmokeProbe) -> Option<String> {
    if probe.render_p95_ns > REAL_SHELL_RENDER_P95_BUDGET_NS {
        return Some(format!(
            "real-shell render p95 exceeded 144Hz frame budget: measured {} ns, budget {} ns",
            probe.render_p95_ns, REAL_SHELL_RENDER_P95_BUDGET_NS
        ));
    }
    if probe.input_to_render_p95_ns > REAL_SHELL_INPUT_TO_RENDER_P95_BUDGET_NS {
        return Some(format!(
            "real-shell input-to-render p95 exceeded latency budget: measured {} ns, budget {} ns",
            probe.input_to_render_p95_ns, REAL_SHELL_INPUT_TO_RENDER_P95_BUDGET_NS
        ));
    }
    None
}
