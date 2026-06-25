use super::RuntimeConfigReloadSmokeReport;
use crate::cli::CliExit;

pub(super) fn runtime_config_reload_smoke_success(
    report: &RuntimeConfigReloadSmokeReport,
) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "runtime config reload smoke: ok\nunchanged poll changed: {}\nchanged poll changed: {}\nterminal: {}x{}\nscrollback lines: {}\ntarget fps: {}\ndirty-region rendering: {}\nfont size px: {}\ncell width px: {}\nline height px: {}\ncell spacing px: {}\nshell: {}\n",
            report.unchanged_poll_changed,
            report.changed_poll_changed,
            report.cols,
            report.rows,
            report.scrollback_lines,
            report.target_fps,
            report.dirty_regions,
            report.font_size_px,
            report.cell_width_px,
            report.line_height_px,
            report.cell_spacing_px,
            report.shell_program,
        ),
        stderr: String::new(),
    }
}

pub(super) fn runtime_config_reload_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime config reload smoke failed: {error}\n"),
    }
}
