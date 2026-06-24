use winit::keyboard::{Key, ModifiersState};

use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use crate::cli::CliExit;
use crate::pty::ShellCommand;
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::pty_smoke::RuntimePerfSmokePtySpawner;

pub(in crate::cli) use idle::{runtime_idle_cpu_smoke_exit, runtime_idle_smoke_exit};

mod idle;

const RUNTIME_RENDER_P95_BUDGET_NS: u64 = 6_940_000;
const RUNTIME_INPUT_TO_RENDER_P95_BUDGET_NS: u64 = 10_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimePerfProbe {
    pumped_bytes: usize,
    metrics: crate::app::NativeRuntimePerfSnapshot,
}

pub(in crate::cli) fn runtime_perf_smoke_exit() -> CliExit {
    let probe = match run_runtime_perf_probe() {
        Ok(probe) => probe,
        Err(error) => return runtime_perf_smoke_error(error),
    };
    runtime_perf_smoke_success(probe)
}

pub(in crate::cli) fn runtime_perf_budget_smoke_exit() -> CliExit {
    let probe = match run_runtime_perf_probe() {
        Ok(probe) => probe,
        Err(error) => return runtime_perf_budget_smoke_error(error),
    };
    let Some(failure) = runtime_perf_budget_failure(&probe) else {
        return CliExit {
            code: 0,
            stdout: format!(
                "runtime perf budget smoke: ok\npumped bytes: {}\nrendered frames: {}\nrender p95 ns: {}\nrender p95 budget ns: {}\ninput-to-render p95 ns: {}\ninput-to-render p95 budget ns: {}\n",
                probe.pumped_bytes,
                probe.metrics.rendered_frames,
                probe.metrics.render_time_p95_ns,
                RUNTIME_RENDER_P95_BUDGET_NS,
                probe.metrics.input_to_render_p95_ns,
                RUNTIME_INPUT_TO_RENDER_P95_BUDGET_NS
            ),
            stderr: String::new(),
        };
    };

    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime perf budget smoke failed: {failure}\n"),
    }
}

fn run_runtime_perf_probe() -> Result<RuntimePerfProbe, String> {
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 24,
        terminal_rows: 4,
        scrollback_lines: 128,
        pixel_width: 0,
        pixel_height: 0,
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

    let key = Key::Character("x".into());
    let sent = match runtime.send_winit_key_input(&key, ModifiersState::empty()) {
        Ok(sent) => sent,
        Err(error) => return Err(error.to_string()),
    };
    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return Err(error.to_string()),
    };
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return Err(error.to_string()),
    };
    let rendered = match runtime.render_terminal_frame(&mut renderer) {
        Ok(rendered) => rendered,
        Err(error) => return Err(error.to_string()),
    };
    let metrics = runtime.dump_runtime_perf_metrics();

    if !sent
        || pumped_bytes == 0
        || !rendered
        || metrics.rendered_dirty_regions == 0
        || metrics.rendered_dirty_cells == 0
        || metrics.render_time_samples == 0
        || metrics.input_to_render_samples == 0
    {
        return Err("input echo did not reach a rendered frame".to_owned());
    }

    Ok(RuntimePerfProbe {
        pumped_bytes,
        metrics,
    })
}

fn runtime_perf_smoke_success(probe: RuntimePerfProbe) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "runtime perf smoke: ok\npumped bytes: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells: {}\nrendered dirty cells max: {}\nrender samples: {}\nrender avg ns: {}\nrender max ns: {}\nrender p95 ns: {}\ninput-to-render samples: {}\ninput-to-render avg ns: {}\ninput-to-render max ns: {}\ninput-to-render p95 ns: {}\n",
            probe.pumped_bytes,
            probe.metrics.rendered_frames,
            probe.metrics.rendered_dirty_regions,
            probe.metrics.rendered_dirty_cells,
            probe.metrics.rendered_dirty_cells_max,
            probe.metrics.render_time_samples,
            probe.metrics.render_time_avg_ns,
            probe.metrics.render_time_max_ns,
            probe.metrics.render_time_p95_ns,
            probe.metrics.input_to_render_samples,
            probe.metrics.input_to_render_avg_ns,
            probe.metrics.input_to_render_max_ns,
            probe.metrics.input_to_render_p95_ns
        ),
        stderr: String::new(),
    }
}

fn runtime_perf_budget_failure(probe: &RuntimePerfProbe) -> Option<&'static str> {
    if probe.metrics.render_time_p95_ns > RUNTIME_RENDER_P95_BUDGET_NS {
        return Some("render p95 exceeded 144Hz frame budget");
    }
    if probe.metrics.input_to_render_p95_ns > RUNTIME_INPUT_TO_RENDER_P95_BUDGET_NS {
        return Some("input-to-render p95 exceeded latency budget");
    }
    None
}

fn runtime_perf_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime perf smoke failed: {error}\n"),
    }
}

fn runtime_perf_budget_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime perf budget smoke failed: {error}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_perf_budget_accepts_p95_values_within_limits() {
        let probe = RuntimePerfProbe {
            pumped_bytes: 1,
            metrics: crate::app::NativeRuntimePerfSnapshot {
                render_time_p95_ns: RUNTIME_RENDER_P95_BUDGET_NS,
                input_to_render_p95_ns: RUNTIME_INPUT_TO_RENDER_P95_BUDGET_NS,
                ..Default::default()
            },
        };

        assert_eq!(runtime_perf_budget_failure(&probe), None);
    }

    #[test]
    fn runtime_perf_budget_rejects_render_p95_over_144hz_budget() {
        let probe = RuntimePerfProbe {
            pumped_bytes: 1,
            metrics: crate::app::NativeRuntimePerfSnapshot {
                render_time_p95_ns: RUNTIME_RENDER_P95_BUDGET_NS + 1,
                input_to_render_p95_ns: RUNTIME_INPUT_TO_RENDER_P95_BUDGET_NS,
                ..Default::default()
            },
        };

        assert_eq!(
            runtime_perf_budget_failure(&probe),
            Some("render p95 exceeded 144Hz frame budget")
        );
    }

    #[test]
    fn runtime_perf_budget_rejects_input_to_render_p95_over_latency_budget() {
        let probe = RuntimePerfProbe {
            pumped_bytes: 1,
            metrics: crate::app::NativeRuntimePerfSnapshot {
                render_time_p95_ns: RUNTIME_RENDER_P95_BUDGET_NS,
                input_to_render_p95_ns: RUNTIME_INPUT_TO_RENDER_P95_BUDGET_NS + 1,
                ..Default::default()
            },
        };

        assert_eq!(
            runtime_perf_budget_failure(&probe),
            Some("input-to-render p95 exceeded latency budget")
        );
    }
}
