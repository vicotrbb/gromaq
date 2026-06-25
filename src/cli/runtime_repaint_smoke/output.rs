use crate::cli::CliExit;

use super::RuntimeRepaintSmokeReport;

pub(super) fn runtime_repaint_smoke_success(report: &RuntimeRepaintSmokeReport) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "runtime repaint smoke: ok\npumped bytes: {}\nrendered: {}\nfull viewport repainted: {}\ncommand preserved: {}\nfirst output row preserved: {}\nsecond output row preserved: {}\nprompt preserved: {}\nplanned glyphs: {}\nclear regions: {}\n",
            report.pumped_bytes,
            report.rendered,
            report.full_viewport_repainted,
            report.command_preserved,
            report.first_output_row_preserved,
            report.second_output_row_preserved,
            report.prompt_preserved,
            report.planned_glyphs,
            report.clear_regions
        ),
        stderr: String::new(),
    }
}

pub(super) fn runtime_repaint_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime repaint smoke failed: {error}\n"),
    }
}
