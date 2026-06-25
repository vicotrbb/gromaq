use super::*;

fn probe(render_p95_ns: u64, input_to_render_p95_ns: u64) -> RuntimeRealShellSmokeProbe {
    RuntimeRealShellSmokeProbe {
        pumped_bytes: 1,
        pty_input_writes: 1,
        pty_input_bytes: 1,
        rendered_frames: 1,
        rendered_dirty_regions: 1,
        rendered_dirty_cells_max: 1,
        ready_observed: true,
        input_echo_observed: true,
        exit_echo_observed: true,
        render_p95_ns,
        input_to_render_p95_ns,
    }
}

#[test]
fn real_shell_perf_budget_accepts_values_at_limit() {
    assert_eq!(
        real_shell_perf_budget_failure(&probe(
            REAL_SHELL_RENDER_P95_BUDGET_NS,
            REAL_SHELL_INPUT_TO_RENDER_P95_BUDGET_NS
        )),
        None
    );
}

#[test]
fn real_shell_perf_budget_rejects_render_p95_over_limit() {
    assert_eq!(
        real_shell_perf_budget_failure(&probe(
            REAL_SHELL_RENDER_P95_BUDGET_NS + 1,
            REAL_SHELL_INPUT_TO_RENDER_P95_BUDGET_NS
        )),
        Some("real-shell render p95 exceeded 144Hz frame budget")
    );
}

#[test]
fn real_shell_perf_budget_rejects_input_to_render_p95_over_limit() {
    assert_eq!(
        real_shell_perf_budget_failure(&probe(
            REAL_SHELL_RENDER_P95_BUDGET_NS,
            REAL_SHELL_INPUT_TO_RENDER_P95_BUDGET_NS + 1
        )),
        Some("real-shell input-to-render p95 exceeded latency budget")
    );
}
