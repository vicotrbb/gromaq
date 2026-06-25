use super::RuntimeRealShellLargeOutputSmokeProbe;

pub(super) const REAL_SHELL_LARGE_OUTPUT_RENDER_P95_BUDGET_NS: u64 = 6_940_000;

pub(super) fn real_shell_large_output_budget_failure(
    probe: &RuntimeRealShellLargeOutputSmokeProbe,
) -> Option<&'static str> {
    if probe.render_p95_ns > REAL_SHELL_LARGE_OUTPUT_RENDER_P95_BUDGET_NS {
        return Some("real-shell large-output render p95 exceeded 144Hz frame budget");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::runtime_real_shell_smoke::REAL_SHELL_LARGE_OUTPUT_SCROLLBACK_LINES;

    fn probe(render_p95_ns: u64) -> RuntimeRealShellLargeOutputSmokeProbe {
        RuntimeRealShellLargeOutputSmokeProbe {
            pumped_bytes: 1,
            scrollback_lines: REAL_SHELL_LARGE_OUTPUT_SCROLLBACK_LINES,
            rendered_frames: 1,
            rendered_dirty_regions: 1,
            rendered_dirty_cells_max: 1,
            first_line_evicted: true,
            last_line_observed: true,
            render_p95_ns,
        }
    }

    #[test]
    fn real_shell_large_output_budget_accepts_render_p95_at_limit() {
        assert_eq!(
            real_shell_large_output_budget_failure(&probe(
                REAL_SHELL_LARGE_OUTPUT_RENDER_P95_BUDGET_NS
            )),
            None
        );
    }

    #[test]
    fn real_shell_large_output_budget_rejects_render_p95_over_limit() {
        assert_eq!(
            real_shell_large_output_budget_failure(&probe(
                REAL_SHELL_LARGE_OUTPUT_RENDER_P95_BUDGET_NS + 1
            )),
            Some("real-shell large-output render p95 exceeded 144Hz frame budget")
        );
    }
}
