use crate::cli::CliExit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeLargeOutputSmokeReport {
    pub(super) lines: usize,
    pub(super) pumped_bytes: usize,
    pub(super) scrollback_lines: usize,
    pub(super) rendered_frames: u64,
    pub(super) rendered_dirty_regions: u64,
    pub(super) rendered_dirty_cells: u64,
    pub(super) rendered_dirty_cells_max: u64,
    pub(super) last_line: String,
    pub(super) render_p95_ns: u64,
}

pub(super) fn runtime_large_output_smoke_success(
    report: &RuntimeLargeOutputSmokeReport,
) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "runtime large-output smoke: ok\nlines: {}\npumped bytes: {}\nscrollback lines: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells: {}\nrendered dirty cells max: {}\nlast visible line: {}\nrender p95 ns: {}\n",
            report.lines,
            report.pumped_bytes,
            report.scrollback_lines,
            report.rendered_frames,
            report.rendered_dirty_regions,
            report.rendered_dirty_cells,
            report.rendered_dirty_cells_max,
            report.last_line,
            report.render_p95_ns
        ),
        stderr: String::new(),
    }
}

pub(super) fn runtime_large_output_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime large-output smoke failed: {error}\n"),
    }
}

pub(super) fn runtime_large_output_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime large-output smoke failed: {reason}\n"),
    }
}
