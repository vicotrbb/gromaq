use crate::cli::CliExit;

pub(in crate::cli) use idle::{runtime_idle_cpu_smoke_exit, runtime_idle_smoke_exit};
use probe::{RuntimePerfProbe, run_runtime_perf_probe};

mod idle;
mod probe;

const RUNTIME_RENDER_P95_BUDGET_NS: u64 = 6_940_000;
const RUNTIME_INPUT_TO_RENDER_P95_BUDGET_NS: u64 = 10_000_000;
const RUNTIME_PERF_P95_SMOKE_SAMPLES: usize = 16;

pub(in crate::cli) fn runtime_perf_smoke_exit() -> CliExit {
    let probe = match run_runtime_perf_probe(1) {
        Ok(probe) => probe,
        Err(error) => return runtime_perf_smoke_error(error),
    };
    runtime_perf_smoke_success(probe)
}

pub(in crate::cli) fn runtime_perf_budget_smoke_exit() -> CliExit {
    let probe = match run_runtime_perf_probe(1) {
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

pub(in crate::cli) fn runtime_perf_p95_smoke_exit() -> CliExit {
    let probe = match run_runtime_perf_probe(RUNTIME_PERF_P95_SMOKE_SAMPLES) {
        Ok(probe) => probe,
        Err(error) => return runtime_perf_p95_smoke_error(error),
    };
    let Some(failure) = runtime_perf_budget_failure(&probe) else {
        return CliExit {
            code: 0,
            stdout: format!(
                "runtime perf p95 smoke: ok\nsamples: {}\npumped bytes: {}\nrendered frames: {}\nrender p95 ns: {}\nrender p95 budget ns: {}\ninput-to-render p95 ns: {}\ninput-to-render p95 budget ns: {}\nrender max ns: {}\ninput-to-render max ns: {}\n",
                probe.expected_samples,
                probe.pumped_bytes,
                probe.metrics.rendered_frames,
                probe.metrics.render_time_p95_ns,
                RUNTIME_RENDER_P95_BUDGET_NS,
                probe.metrics.input_to_render_p95_ns,
                RUNTIME_INPUT_TO_RENDER_P95_BUDGET_NS,
                probe.metrics.render_time_max_ns,
                probe.metrics.input_to_render_max_ns
            ),
            stderr: String::new(),
        };
    };

    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime perf p95 smoke failed: {failure}\n"),
    }
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

fn runtime_perf_p95_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime perf p95 smoke failed: {error}\n"),
    }
}

#[cfg(test)]
mod tests;
