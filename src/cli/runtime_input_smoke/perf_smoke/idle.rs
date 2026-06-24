use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use crate::cli::CliExit;
use crate::pty::ShellCommand;
use crate::renderer::{RendererConfig, WgpuRenderer};

use crate::cli::runtime_input_smoke::pty_smoke::RuntimePerfSmokePtySpawner;

const RUNTIME_IDLE_SMOKE_RENDER_ATTEMPTS: u64 = 16;
const RUNTIME_IDLE_CPU_SMOKE_SAMPLES: usize = 5;
const RUNTIME_IDLE_CPU_SMOKE_SAMPLE_INTERVAL_MS: u64 = 50;
const RUNTIME_IDLE_CPU_BUDGET_PERCENT: f32 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeIdleProbe {
    pumped_bytes: usize,
    metrics: crate::app::NativeRuntimePerfSnapshot,
}

pub(in crate::cli) fn runtime_idle_smoke_exit() -> CliExit {
    let probe = match run_runtime_idle_probe() {
        Ok(probe) => probe,
        Err(error) => return runtime_idle_smoke_error(error),
    };
    runtime_idle_smoke_success(probe)
}

pub(in crate::cli) fn runtime_idle_cpu_smoke_exit() -> CliExit {
    let probe = match run_runtime_idle_probe() {
        Ok(probe) => probe,
        Err(error) => return runtime_idle_cpu_smoke_error(error),
    };
    let mut max_cpu_percent = 0.0_f32;
    for _ in 0..RUNTIME_IDLE_CPU_SMOKE_SAMPLES {
        sleep(Duration::from_millis(
            RUNTIME_IDLE_CPU_SMOKE_SAMPLE_INTERVAL_MS,
        ));
        let cpu_percent = match current_process_cpu_percent() {
            Ok(cpu_percent) => cpu_percent,
            Err(error) => return runtime_idle_cpu_smoke_error(error),
        };
        max_cpu_percent = max_cpu_percent.max(cpu_percent);
    }

    if let Some(failure) = runtime_idle_cpu_budget_failure(max_cpu_percent) {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("runtime idle cpu smoke failed: {failure}\n"),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime idle cpu smoke: ok\nsamples: {}\nsample interval ms: {}\nmax cpu percent: {:.1}\ncpu budget percent: {:.1}\nrender attempts: {}\nclean frame skips: {}\nrendered frames: {}\n",
            RUNTIME_IDLE_CPU_SMOKE_SAMPLES,
            RUNTIME_IDLE_CPU_SMOKE_SAMPLE_INTERVAL_MS,
            max_cpu_percent,
            RUNTIME_IDLE_CPU_BUDGET_PERCENT,
            probe.metrics.render_attempts,
            probe.metrics.clean_frame_skips,
            probe.metrics.rendered_frames
        ),
        stderr: String::new(),
    }
}

fn run_runtime_idle_probe() -> Result<RuntimeIdleProbe, String> {
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

fn current_process_cpu_percent() -> Result<f32, String> {
    let pid = std::process::id().to_string();
    let output = Command::new("ps")
        .args(["-o", "%cpu=", "-p", &pid])
        .output()
        .map_err(|error| format!("process cpu sampling failed to start: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "process cpu sampling failed with status {}",
            output.status
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("process cpu output was not utf-8: {error}"))?;
    stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| "process cpu output was empty".to_owned())?
        .parse::<f32>()
        .map_err(|error| format!("process cpu output was not numeric: {error}"))
}

fn runtime_idle_cpu_budget_failure(max_cpu_percent: f32) -> Option<&'static str> {
    if max_cpu_percent > RUNTIME_IDLE_CPU_BUDGET_PERCENT {
        return Some("idle cpu percent exceeded budget");
    }
    None
}

fn runtime_idle_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime idle smoke failed: {error}\n"),
    }
}

fn runtime_idle_cpu_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime idle cpu smoke failed: {error}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_idle_cpu_budget_accepts_cpu_at_limit() {
        assert_eq!(
            runtime_idle_cpu_budget_failure(RUNTIME_IDLE_CPU_BUDGET_PERCENT),
            None
        );
    }

    #[test]
    fn runtime_idle_cpu_budget_rejects_cpu_over_limit() {
        assert_eq!(
            runtime_idle_cpu_budget_failure(RUNTIME_IDLE_CPU_BUDGET_PERCENT + 0.1),
            Some("idle cpu percent exceeded budget")
        );
    }
}
