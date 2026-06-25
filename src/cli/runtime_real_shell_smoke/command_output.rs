use std::thread;
use std::time::Instant;

use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, RealNativePtySpawner};
use crate::cli::CliExit;
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::support::{real_shell_command_output_command, runtime_transcript};
use super::{
    REAL_SHELL_COMMAND_OUTPUT_FIRST, REAL_SHELL_COMMAND_OUTPUT_INPUT,
    REAL_SHELL_COMMAND_OUTPUT_PROMPT, REAL_SHELL_COMMAND_OUTPUT_SECOND, REAL_SHELL_EXIT,
    REAL_SHELL_READY, REAL_SHELL_SMOKE_COLS, REAL_SHELL_SMOKE_POLL_INTERVAL, REAL_SHELL_SMOKE_ROWS,
    REAL_SHELL_SMOKE_TIMEOUT,
};

pub(in crate::cli) fn runtime_real_shell_command_output_smoke_exit() -> CliExit {
    let probe = match run_runtime_real_shell_command_output_smoke() {
        Ok(probe) => probe,
        Err(error) => return runtime_real_shell_command_output_smoke_error(error),
    };

    CliExit {
        code: 0,
        stdout: format!(
            "runtime real-shell command-output smoke: ok\nshell: /bin/sh\npumped bytes: {}\npty input writes: {}\npty input bytes: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells max: {}\ncommand output observed: {}\nprompt observed: {}\nfull redraw preserved output: {}\nrender p95 ns: {}\n",
            probe.pumped_bytes,
            probe.pty_input_writes,
            probe.pty_input_bytes,
            probe.rendered_frames,
            probe.rendered_dirty_regions,
            probe.rendered_dirty_cells_max,
            probe.command_output_observed,
            probe.prompt_observed,
            probe.full_redraw_preserved_output,
            probe.render_p95_ns
        ),
        stderr: String::new(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeRealShellCommandOutputSmokeProbe {
    pumped_bytes: usize,
    pty_input_writes: u64,
    pty_input_bytes: u64,
    rendered_frames: u64,
    rendered_dirty_regions: u64,
    rendered_dirty_cells_max: u64,
    command_output_observed: bool,
    prompt_observed: bool,
    full_redraw_preserved_output: bool,
    render_p95_ns: u64,
}

type RuntimeRealShellCommandOutputResult = Result<RuntimeRealShellCommandOutputSmokeProbe, String>;

fn run_runtime_real_shell_command_output_smoke() -> RuntimeRealShellCommandOutputResult {
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: REAL_SHELL_SMOKE_COLS,
        terminal_rows: REAL_SHELL_SMOKE_ROWS,
        scrollback_lines: 64,
        pixel_width: 0,
        pixel_height: 0,
        cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
        cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
        shell: real_shell_command_output_command(),
    })
    .map_err(|error| error.to_string())?;
    runtime
        .start_shell(&RealNativePtySpawner::default())
        .map_err(|error| error.to_string())?;
    runtime
        .send_pty_input(
            format!("{REAL_SHELL_COMMAND_OUTPUT_INPUT}\n{REAL_SHELL_EXIT}\n").as_bytes(),
        )
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
        let command_output_observed = transcript.contains(REAL_SHELL_COMMAND_OUTPUT_FIRST)
            && transcript.contains(REAL_SHELL_COMMAND_OUTPUT_SECOND);
        let prompt_observed = transcript.contains(REAL_SHELL_COMMAND_OUTPUT_PROMPT);
        if transcript.contains(REAL_SHELL_READY) && command_output_observed && prompt_observed {
            runtime.invalidate_terminal_frame();
            let full_redraw_rendered = runtime
                .render_terminal_frame(&mut renderer)
                .map_err(|error| error.to_string())?;
            let planned_text = renderer
                .last_plan()
                .map(|plan| {
                    plan.glyphs
                        .iter()
                        .map(|glyph| glyph.text.as_str())
                        .collect::<String>()
                })
                .unwrap_or_default();
            let full_redraw_preserved_output = full_redraw_rendered
                && planned_text.contains(REAL_SHELL_COMMAND_OUTPUT_FIRST)
                && planned_text.contains(REAL_SHELL_COMMAND_OUTPUT_SECOND)
                && planned_text.contains(REAL_SHELL_COMMAND_OUTPUT_PROMPT);
            if !full_redraw_preserved_output {
                return Err(format!(
                    "full redraw dropped real shell command output; planned={planned_text:?}"
                ));
            }

            let metrics = runtime.dump_runtime_perf_metrics();
            if metrics.rendered_frames == 0
                || metrics.rendered_dirty_regions == 0
                || metrics.rendered_dirty_cells_max == 0
            {
                return Err("real shell command output did not render dirty frames".to_owned());
            }
            return Ok(RuntimeRealShellCommandOutputSmokeProbe {
                pumped_bytes,
                pty_input_writes: metrics.pty_input_writes,
                pty_input_bytes: metrics.pty_input_bytes,
                rendered_frames: metrics.rendered_frames,
                rendered_dirty_regions: metrics.rendered_dirty_regions,
                rendered_dirty_cells_max: metrics.rendered_dirty_cells_max,
                command_output_observed,
                prompt_observed,
                full_redraw_preserved_output,
                render_p95_ns: metrics.render_time_p95_ns,
            });
        }

        if Instant::now() >= deadline {
            return Err(format!(
                "timed out waiting for real shell command output; observed: {}",
                transcript.replace('\n', "|")
            ));
        }
        thread::sleep(REAL_SHELL_SMOKE_POLL_INTERVAL);
    }
}

fn runtime_real_shell_command_output_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime real-shell command-output smoke failed: {error}\n"),
    }
}
