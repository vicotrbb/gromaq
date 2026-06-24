use std::ffi::OsString;
use std::thread;
use std::time::{Duration, Instant};

use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, RealNativePtySpawner};
use crate::cli::CliExit;
use crate::pty::ShellCommand;
use crate::renderer::{RendererConfig, WgpuRenderer};

const REAL_SHELL_SMOKE_COLS: u16 = 48;
const REAL_SHELL_SMOKE_ROWS: u16 = 8;
const REAL_SHELL_SMOKE_TIMEOUT: Duration = Duration::from_secs(3);
const REAL_SHELL_SMOKE_POLL_INTERVAL: Duration = Duration::from_millis(10);
const REAL_SHELL_READY: &str = "gromaq-real-shell-ready";
const REAL_SHELL_INPUT: &str = "gromaq-real-shell-input";
const REAL_SHELL_EXIT: &str = "gromaq-real-shell-exit";
const REAL_SHELL_LARGE_OUTPUT_LINES: usize = 256;
const REAL_SHELL_LARGE_OUTPUT_SCROLLBACK_LINES: usize = 64;

pub(in crate::cli) fn runtime_real_shell_smoke_exit() -> CliExit {
    let probe = match run_runtime_real_shell_smoke() {
        Ok(probe) => probe,
        Err(error) => return runtime_real_shell_smoke_error(error),
    };

    CliExit {
        code: 0,
        stdout: format!(
            "runtime real-shell smoke: ok\nshell: /bin/sh\npumped bytes: {}\npty input writes: {}\npty input bytes: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells max: {}\nready observed: {}\ninput echo observed: {}\nexit echo observed: {}\nrender p95 ns: {}\ninput-to-render p95 ns: {}\n",
            probe.pumped_bytes,
            probe.pty_input_writes,
            probe.pty_input_bytes,
            probe.rendered_frames,
            probe.rendered_dirty_regions,
            probe.rendered_dirty_cells_max,
            probe.ready_observed,
            probe.input_echo_observed,
            probe.exit_echo_observed,
            probe.render_p95_ns,
            probe.input_to_render_p95_ns
        ),
        stderr: String::new(),
    }
}

pub(in crate::cli) fn runtime_real_shell_large_output_smoke_exit() -> CliExit {
    let probe = match run_runtime_real_shell_large_output_smoke() {
        Ok(probe) => probe,
        Err(error) => return runtime_real_shell_large_output_smoke_error(error),
    };

    CliExit {
        code: 0,
        stdout: format!(
            "runtime real-shell large-output smoke: ok\nshell: /bin/sh\nlines: {}\npumped bytes: {}\nscrollback cap: {}\nscrollback lines: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells max: {}\nfirst line evicted: {}\nlast line observed: {}\nrender p95 ns: {}\n",
            REAL_SHELL_LARGE_OUTPUT_LINES,
            probe.pumped_bytes,
            REAL_SHELL_LARGE_OUTPUT_SCROLLBACK_LINES,
            probe.scrollback_lines,
            probe.rendered_frames,
            probe.rendered_dirty_regions,
            probe.rendered_dirty_cells_max,
            probe.first_line_evicted,
            probe.last_line_observed,
            probe.render_p95_ns
        ),
        stderr: String::new(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeRealShellLargeOutputSmokeProbe {
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

fn run_runtime_real_shell_smoke() -> Result<RuntimeRealShellSmokeProbe, String> {
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: REAL_SHELL_SMOKE_COLS,
        terminal_rows: REAL_SHELL_SMOKE_ROWS,
        scrollback_lines: 64,
        pixel_width: 0,
        pixel_height: 0,
        shell: real_shell_command(),
    })
    .map_err(|error| error.to_string())?;
    runtime
        .start_shell(&RealNativePtySpawner::default())
        .map_err(|error| error.to_string())?;
    runtime
        .send_pty_input(format!("{REAL_SHELL_INPUT}\n{REAL_SHELL_EXIT}\n").as_bytes())
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

fn run_runtime_real_shell_large_output_smoke() -> RuntimeRealShellLargeOutputResult {
    let first_line = real_shell_large_output_line(0);
    let last_line = real_shell_large_output_line(REAL_SHELL_LARGE_OUTPUT_LINES - 1);
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: REAL_SHELL_SMOKE_COLS,
        terminal_rows: REAL_SHELL_SMOKE_ROWS,
        scrollback_lines: REAL_SHELL_LARGE_OUTPUT_SCROLLBACK_LINES,
        pixel_width: 0,
        pixel_height: 0,
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

fn real_shell_command() -> ShellCommand {
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

fn real_shell_large_output_command() -> ShellCommand {
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

fn real_shell_large_output_line(line: usize) -> String {
    format!("gromaq-real-large-line-{line:03}")
}

fn runtime_transcript<S>(runtime: &NativeTerminalRuntime<S>) -> String {
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

fn runtime_real_shell_large_output_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime real-shell large-output smoke failed: {error}\n"),
    }
}

fn runtime_real_shell_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime real-shell smoke failed: {error}\n"),
    }
}
