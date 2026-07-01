use crate::app::NativeAppRunReport;
use crate::cli::CliExit;

mod perf;

pub(super) use perf::{window_perf_no_glyph_failure, window_perf_success};

const DEFAULT_STARTUP_MARKER: &str = "tmux Cmd/Ctrl+Shift+T";

pub(super) fn window_smoke_success(report: &NativeAppRunReport) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "window smoke: ok\npresented frame limit: 3\nredraw attempts: {}\nframes presented: {}\nsurface timeouts: {}\nsurface occluded: {}\nterminal cells: {}x{}\ndefault startup content checked: {}\ndefault startup marker: {DEFAULT_STARTUP_MARKER}\ntmux status strip rendered: {}\ntmux status pane command rendered: {}\ntmux manager panel rendered: {}\n",
            report.redraw_attempts,
            report.frames_presented,
            report.surface_frame_timeouts,
            report.surface_frame_occluded,
            report.terminal_cols,
            report.terminal_rows,
            report.default_startup_content_checked,
            report.tmux_status_strip_rendered,
            report.tmux_status_pane_command_rendered,
            report.tmux_manager_panel_rendered
        ),
        stderr: String::new(),
    }
}

pub(super) fn window_screenshot_smoke_success(
    report: &NativeAppRunReport,
    frame_limit: u64,
) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "window screenshot smoke: ok\npresented frame limit: {frame_limit}\nredraw attempts: {}\nframes presented: {}\nsurface timeouts: {}\nsurface occluded: {}\ndefault startup content checked: {}\ndefault startup marker: {DEFAULT_STARTUP_MARKER}\ntmux status strip rendered: {}\ntmux manager panel rendered: {}\nglyph frame presented: {}\nglyph frame glyph quads: {}\nglyph frame cursor quads: {}\n",
            report.redraw_attempts,
            report.frames_presented,
            report.surface_frame_timeouts,
            report.surface_frame_occluded,
            report.default_startup_content_checked,
            report.tmux_status_strip_rendered,
            report.tmux_manager_panel_rendered,
            report.glyph_frame_presented,
            report.glyph_frame_glyph_quads,
            report.glyph_frame_cursor_quads
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

pub(super) fn window_smoke_no_default_tmux_ui_failure(report: &NativeAppRunReport) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!(
            "window smoke failed: default tmux UI was not rendered; default startup content checked: {}; tmux status strip rendered: {}; tmux status pane command rendered: {}; tmux manager panel rendered: {}\n",
            report.default_startup_content_checked,
            report.tmux_status_strip_rendered,
            report.tmux_status_pane_command_rendered,
            report.tmux_manager_panel_rendered
        ),
    }
}

pub(super) fn window_screenshot_smoke_no_default_tmux_ui_failure(
    report: &NativeAppRunReport,
) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!(
            "window screenshot smoke failed: tmux UI was not rendered; default startup content checked: {}; tmux status strip rendered: {}; tmux manager panel rendered: {}\n",
            report.default_startup_content_checked,
            report.tmux_status_strip_rendered,
            report.tmux_manager_panel_rendered
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
            "window glyph frame snapshot: ok\npath: {path}\nbytes written: {}\nframe size: {}x{}\nglyph frame presented: {}\ntmux status strip rendered: {}\ntmux manager panel rendered: {}\nglyph frame glyph quads: {}\nglyph frame background quads: {}\nglyph frame cursor quads: {}\n",
            report.glyph_frame_snapshot_bytes,
            report.glyph_frame_snapshot_width,
            report.glyph_frame_snapshot_height,
            report.glyph_frame_presented,
            report.tmux_status_strip_rendered,
            report.tmux_manager_panel_rendered,
            report.glyph_frame_glyph_quads,
            report.glyph_frame_background_quads,
            report.glyph_frame_cursor_quads,
        ),
        stderr: String::new(),
    }
}

pub(super) fn window_tmux_manager_snapshot_success(
    path: &str,
    report: &NativeAppRunReport,
) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "window tmux manager snapshot: ok\npath: {path}\ndefault startup content checked: {}\ndefault startup marker: {DEFAULT_STARTUP_MARKER}\nbytes written: {}\nframe size: {}x{}\nterminal cells: {}x{}\nglyph frame presented: {}\ntmux status strip rendered: {}\ntmux status pane command rendered: {}\ntmux manager panel rendered: {}\ntmux manager sessions: {}\ntmux manager windows: {}\ntmux manager panes: {}\nglyph frame glyph quads: {}\nglyph frame background quads: {}\nglyph frame cursor quads: {}\n",
            report.default_startup_content_checked,
            report.glyph_frame_snapshot_bytes,
            report.glyph_frame_snapshot_width,
            report.glyph_frame_snapshot_height,
            report.terminal_cols,
            report.terminal_rows,
            report.glyph_frame_presented,
            report.tmux_status_strip_rendered,
            report.tmux_status_pane_command_rendered,
            report.tmux_manager_panel_rendered,
            report.tmux_manager_sessions,
            report.tmux_manager_windows,
            report.tmux_manager_panes,
            report.glyph_frame_glyph_quads,
            report.glyph_frame_background_quads,
            report.glyph_frame_cursor_quads,
        ),
        stderr: String::new(),
    }
}

pub(super) fn window_tmux_manager_snapshot_failure(report: &NativeAppRunReport) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!(
            "window tmux manager snapshot failed\nsnapshot written: {}\ndefault startup content checked: {}\ntmux status strip rendered: {}\ntmux status pane command rendered: {}\ntmux manager panel rendered: {}\nterminal cells: {}x{}\nframes presented: {}\nglyph frame presented: {}\nglyph quads: {}\nbackground quads: {}\ncursor quads: {}\n",
            report.glyph_frame_snapshot_written,
            report.default_startup_content_checked,
            report.tmux_status_strip_rendered,
            report.tmux_status_pane_command_rendered,
            report.tmux_manager_panel_rendered,
            report.terminal_cols,
            report.terminal_rows,
            report.frames_presented,
            report.glyph_frame_presented,
            report.glyph_frame_glyph_quads,
            report.glyph_frame_background_quads,
            report.glyph_frame_cursor_quads,
        ),
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
