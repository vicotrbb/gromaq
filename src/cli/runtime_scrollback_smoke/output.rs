use crate::cli::CliExit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeScrollbackSmokeReport {
    pub(super) pumped_bytes: usize,
    pub(super) local_scroll_rows: u64,
    pub(super) rendered_frames: u64,
    pub(super) rendered_dirty_regions: u64,
    pub(super) rendered_dirty_cells_max: u64,
    pub(super) scrolled_lines: [String; 3],
    pub(super) live_lines: [String; 3],
    pub(super) pty_input_writes: u64,
}

pub(super) fn runtime_scrollback_smoke_success(report: &RuntimeScrollbackSmokeReport) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "runtime scrollback smoke: ok\npumped bytes: {}\nlocal scroll rows: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells max: {}\nscrolled visible lines: {}|{}|{}\nlive visible lines: {}|{}|{}\npty input writes: {}\n",
            report.pumped_bytes,
            report.local_scroll_rows,
            report.rendered_frames,
            report.rendered_dirty_regions,
            report.rendered_dirty_cells_max,
            report.scrolled_lines[0],
            report.scrolled_lines[1],
            report.scrolled_lines[2],
            report.live_lines[0],
            report.live_lines[1],
            report.live_lines[2],
            report.pty_input_writes
        ),
        stderr: String::new(),
    }
}

pub(super) fn runtime_scrollback_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime scrollback smoke failed: {error}\n"),
    }
}

pub(super) fn runtime_scrollback_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime scrollback smoke failed: {reason}\n"),
    }
}
