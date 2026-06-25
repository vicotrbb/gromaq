use crate::cli::CliExit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeMemorySmokeReport {
    pub(super) warmup_batches: usize,
    pub(super) measured_batches: usize,
    pub(super) total_lines: usize,
    pub(super) pumped_bytes: usize,
    pub(super) scrollback_cap: usize,
    pub(super) scrollback_lines: usize,
    pub(super) scrollback_cells: usize,
    pub(super) scrollback_cell_limit: usize,
    pub(super) rendered_frames: u64,
    pub(super) rendered_dirty_cells_max: u64,
    pub(super) baseline_rss_kib: u64,
    pub(super) peak_rss_kib: u64,
    pub(super) rss_growth_kib: u64,
    pub(super) rss_growth_cap_kib: u64,
    pub(super) last_line: String,
}

pub(super) fn runtime_memory_smoke_success(report: &RuntimeMemorySmokeReport) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "runtime memory smoke: ok\nwarmup batches: {}\nmeasured batches: {}\nlines: {}\npumped bytes: {}\nscrollback cap: {}\nscrollback lines: {}\nscrollback cells: {}\nscrollback max cells: {}\nrendered frames: {}\nrendered dirty cells max: {}\nrss baseline kib: {}\nrss peak kib: {}\nrss growth kib: {}\nrss growth cap kib: {}\nlast visible line: {}\n",
            report.warmup_batches,
            report.measured_batches,
            report.total_lines,
            report.pumped_bytes,
            report.scrollback_cap,
            report.scrollback_lines,
            report.scrollback_cells,
            report.scrollback_cell_limit,
            report.rendered_frames,
            report.rendered_dirty_cells_max,
            report.baseline_rss_kib,
            report.peak_rss_kib,
            report.rss_growth_kib,
            report.rss_growth_cap_kib,
            report.last_line
        ),
        stderr: String::new(),
    }
}

pub(super) fn runtime_memory_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime memory smoke failed: {error}\n"),
    }
}

pub(super) fn runtime_memory_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime memory smoke failed: {reason}\n"),
    }
}
