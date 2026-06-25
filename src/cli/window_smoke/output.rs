use crate::app::NativeAppRunReport;
use crate::cli::CliExit;

mod perf;

pub(super) use perf::{window_perf_no_glyph_failure, window_perf_success};

pub(super) fn window_smoke_success(report: &NativeAppRunReport) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "window smoke: ok\npresented frame limit: 1\nredraw attempts: {}\nsurface timeouts: {}\nsurface occluded: {}\n",
            report.redraw_attempts, report.surface_frame_timeouts, report.surface_frame_occluded
        ),
        stderr: String::new(),
    }
}

pub(super) fn window_smoke_no_surface_failure(report: &NativeAppRunReport) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!(
            "window smoke failed: no surface frame was presented; redraw attempts: {}; surface timeouts: {}; surface occluded: {}\n",
            report.redraw_attempts, report.surface_frame_timeouts, report.surface_frame_occluded
        ),
    }
}

pub(super) fn window_glyph_frame_snapshot_success(
    path: &str,
    report: &NativeAppRunReport,
) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "window glyph frame snapshot: ok\npath: {path}\nbytes written: {}\nframe size: {}x{}\nglyph frame presented: {}\nglyph frame glyph quads: {}\nglyph frame background quads: {}\nglyph frame cursor quads: {}\n",
            report.glyph_frame_snapshot_bytes,
            report.glyph_frame_snapshot_width,
            report.glyph_frame_snapshot_height,
            report.glyph_frame_presented,
            report.glyph_frame_glyph_quads,
            report.glyph_frame_background_quads,
            report.glyph_frame_cursor_quads,
        ),
        stderr: String::new(),
    }
}

pub(super) fn window_glyph_frame_snapshot_failure(report: &NativeAppRunReport) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!(
            "window glyph frame snapshot failed: no snapshot was written; frames presented: {}; glyph frame presented: {}; glyph quads: {}; background quads: {}; cursor quads: {}\n",
            report.frames_presented,
            report.glyph_frame_presented,
            report.glyph_frame_glyph_quads,
            report.glyph_frame_background_quads,
            report.glyph_frame_cursor_quads,
        ),
    }
}
