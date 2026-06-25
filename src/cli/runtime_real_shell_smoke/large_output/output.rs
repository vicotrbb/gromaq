use crate::cli::CliExit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RuntimeRealShellLargeOutputReport {
    pub(super) lines: usize,
    pub(super) pumped_bytes: usize,
    pub(super) scrollback_cap: usize,
    pub(super) scrollback_lines: usize,
    pub(super) rendered_frames: u64,
    pub(super) rendered_dirty_regions: u64,
    pub(super) rendered_dirty_cells_max: u64,
    pub(super) first_line_evicted: bool,
    pub(super) last_line_observed: bool,
    pub(super) render_p95_ns: u64,
    pub(super) render_p95_budget_ns: u64,
}

pub(super) fn runtime_real_shell_large_output_smoke_success(
    report: &RuntimeRealShellLargeOutputReport,
) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "runtime real-shell large-output smoke: ok\nshell: /bin/sh\nlines: {}\npumped bytes: {}\nscrollback cap: {}\nscrollback lines: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells max: {}\nfirst line evicted: {}\nlast line observed: {}\nrender p95 ns: {}\nrender p95 budget ns: {}\n",
            report.lines,
            report.pumped_bytes,
            report.scrollback_cap,
            report.scrollback_lines,
            report.rendered_frames,
            report.rendered_dirty_regions,
            report.rendered_dirty_cells_max,
            report.first_line_evicted,
            report.last_line_observed,
            report.render_p95_ns,
            report.render_p95_budget_ns
        ),
        stderr: String::new(),
    }
}

pub(super) fn runtime_real_shell_large_output_smoke_error(
    error: impl std::fmt::Display,
) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime real-shell large-output smoke failed: {error}\n"),
    }
}

pub(super) fn runtime_real_shell_large_output_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime real-shell large-output smoke failed: {reason}\n"),
    }
}
