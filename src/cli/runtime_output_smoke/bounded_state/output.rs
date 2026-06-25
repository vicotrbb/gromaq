use crate::cli::CliExit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeBoundedStateSmokeReport {
    pub(super) batches: usize,
    pub(super) total_lines: usize,
    pub(super) pumped_bytes: usize,
    pub(super) scrollback_cap: usize,
    pub(super) scrollback_lines: usize,
    pub(super) scrollback_cell_rows: usize,
    pub(super) scrollback_cells: usize,
    pub(super) scrollback_cell_limit: usize,
    pub(super) rendered_frames: u64,
    pub(super) rendered_dirty_regions: u64,
    pub(super) rendered_dirty_cells: u64,
    pub(super) rendered_dirty_cells_max: u64,
    pub(super) last_line: String,
}

pub(super) fn runtime_bounded_state_smoke_success(
    report: &RuntimeBoundedStateSmokeReport,
) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "runtime bounded-state smoke: ok\nbatches: {}\nlines: {}\npumped bytes: {}\nscrollback cap: {}\nscrollback lines: {}\nscrollback cell rows: {}\nscrollback cells: {}\nscrollback max cells: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells: {}\nrendered dirty cells max: {}\nlast visible line: {}\n",
            report.batches,
            report.total_lines,
            report.pumped_bytes,
            report.scrollback_cap,
            report.scrollback_lines,
            report.scrollback_cell_rows,
            report.scrollback_cells,
            report.scrollback_cell_limit,
            report.rendered_frames,
            report.rendered_dirty_regions,
            report.rendered_dirty_cells,
            report.rendered_dirty_cells_max,
            report.last_line
        ),
        stderr: String::new(),
    }
}

pub(super) fn runtime_bounded_state_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime bounded-state smoke failed: {error}\n"),
    }
}

pub(super) fn runtime_bounded_state_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime bounded-state smoke failed: {reason}\n"),
    }
}
