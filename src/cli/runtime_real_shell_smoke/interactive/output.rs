use crate::cli::CliExit;

use super::{
    REAL_SHELL_INPUT_TO_RENDER_P95_BUDGET_NS, REAL_SHELL_RENDER_P95_BUDGET_NS,
    RuntimeRealShellSmokeProbe,
};

pub(super) fn real_shell_smoke_success(probe: &RuntimeRealShellSmokeProbe) -> CliExit {
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

pub(super) fn real_shell_perf_budget_smoke_success(probe: &RuntimeRealShellSmokeProbe) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "runtime real-shell perf budget smoke: ok\nshell: /bin/sh\npumped bytes: {}\nrendered frames: {}\nrender p95 ns: {}\nrender p95 budget ns: {}\ninput-to-render p95 ns: {}\ninput-to-render p95 budget ns: {}\n",
            probe.pumped_bytes,
            probe.rendered_frames,
            probe.render_p95_ns,
            REAL_SHELL_RENDER_P95_BUDGET_NS,
            probe.input_to_render_p95_ns,
            REAL_SHELL_INPUT_TO_RENDER_P95_BUDGET_NS
        ),
        stderr: String::new(),
    }
}

pub(super) fn runtime_real_shell_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime real-shell smoke failed: {error}\n"),
    }
}

pub(super) fn runtime_real_shell_perf_budget_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime real-shell perf budget smoke failed: {error}\n"),
    }
}
