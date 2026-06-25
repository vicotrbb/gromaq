use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use crate::cli::CliExit;
use crate::pty::ShellCommand;
use crate::renderer::{RendererConfig, WgpuRenderer};

use crate::cli::runtime_input_smoke::pty_smoke::RuntimePerfSmokePtySpawner;

mod cpu;

pub(in crate::cli) use cpu::runtime_idle_cpu_smoke_exit;

const RUNTIME_IDLE_SMOKE_RENDER_ATTEMPTS: u64 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RuntimeIdleProbe {
    pumped_bytes: usize,
    pub(super) metrics: crate::app::NativeRuntimePerfSnapshot,
}

pub(in crate::cli) fn runtime_idle_smoke_exit() -> CliExit {
    let probe = match run_runtime_idle_probe() {
        Ok(probe) => probe,
        Err(error) => return runtime_idle_smoke_error(error),
    };
    runtime_idle_smoke_success(probe)
}

pub(super) fn run_runtime_idle_probe() -> Result<RuntimeIdleProbe, String> {
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 24,
        terminal_rows: 4,
        scrollback_lines: 128,
        pixel_width: 0,
        pixel_height: 0,
        cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
        cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return Err(error.to_string()),
    };
    if let Err(error) = runtime.start_shell(&RuntimePerfSmokePtySpawner) {
        return Err(error.to_string());
    }
    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return Err(error.to_string()),
    };
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return Err(error.to_string()),
    };
    for _ in 0..RUNTIME_IDLE_SMOKE_RENDER_ATTEMPTS {
        let rendered = match runtime.render_terminal_frame(&mut renderer) {
            Ok(rendered) => rendered,
            Err(error) => return Err(error.to_string()),
        };
        if rendered {
            return Err("clean runtime produced a rendered frame".to_owned());
        }
    }
    let metrics = runtime.dump_runtime_perf_metrics();
    if pumped_bytes != 0
        || metrics.pty_output_batches != 0
        || metrics.pty_output_bytes != 0
        || metrics.render_attempts != RUNTIME_IDLE_SMOKE_RENDER_ATTEMPTS
        || metrics.clean_frame_skips != RUNTIME_IDLE_SMOKE_RENDER_ATTEMPTS
        || metrics.rendered_frames != 0
        || metrics.render_time_samples != 0
        || metrics.input_to_render_samples != 0
    {
        return Err("idle runtime counters did not prove clean-frame suppression".to_owned());
    }

    Ok(RuntimeIdleProbe {
        pumped_bytes,
        metrics,
    })
}

fn runtime_idle_smoke_success(probe: RuntimeIdleProbe) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "runtime idle smoke: ok\npumped bytes: {}\nrender attempts: {}\nclean frame skips: {}\nrendered frames: {}\n",
            probe.pumped_bytes,
            probe.metrics.render_attempts,
            probe.metrics.clean_frame_skips,
            probe.metrics.rendered_frames
        ),
        stderr: String::new(),
    }
}

fn runtime_idle_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime idle smoke failed: {error}\n"),
    }
}
