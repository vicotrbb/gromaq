use std::time::Instant;

use super::args::{CliCommand, usage};
use super::{CliExit, NativeAppLaunchConfig, NativeAppLauncher};
use crate::pty::ShellCommand;

const WINDOW_PERF_SMOKE_FRAME_LIMIT: u64 = 180;

pub(super) fn window_smoke_exit<A>(command: CliCommand<'_>, app_launcher: Option<&A>) -> CliExit
where
    A: NativeAppLauncher,
{
    let Some(app_launcher) = app_launcher else {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!(
                "{}native app launch unavailable for {}\n",
                usage(),
                window_smoke_command_name(command),
            ),
        };
    };

    let mut launch_config = NativeAppLaunchConfig::default();
    let frame_limit = match command {
        CliCommand::WindowSmoke => 1,
        CliCommand::WindowPerfSmoke => WINDOW_PERF_SMOKE_FRAME_LIMIT,
        _ => unreachable!(),
    };
    launch_config.app.exit_after_presented_frames = Some(frame_limit);
    if command == CliCommand::WindowPerfSmoke {
        launch_config.app.redraw_until_presented_frame_limit = true;
        launch_config.runtime.shell = ShellCommand {
            program: "/bin/sh".into(),
            args: vec![
                "-lc".into(),
                "printf 'gromaq window perf smoke\\nframe pacing probe\\n'".into(),
            ],
            cwd: None,
        };
    }

    let target_fps = launch_config.app.target_fps;
    let started_at = Instant::now();
    match app_launcher.launch(launch_config) {
        Ok(report) => {
            if command == CliCommand::WindowPerfSmoke {
                let monitor_refresh_millihertz = report
                    .monitor_refresh_millihertz
                    .map(|refresh| refresh.to_string())
                    .unwrap_or_else(|| "unknown".to_owned());
                let surface_present_mode = report.surface_present_mode.unwrap_or("unknown");
                let window_size =
                    format_window_size(report.window_width_px, report.window_height_px);
                let window_scale = report
                    .window_scale_milliscale
                    .map(|scale| scale.to_string())
                    .unwrap_or_else(|| "unknown".to_owned());
                let frame_interval_target_ns =
                    1_000_000_000 / u64::from(report.frame_interval_target_fps.max(1));
                let frame_pacing_accepted = window_frame_pacing_accepted(&report);
                CliExit {
                    code: 0,
                    stdout: format!(
                        "window perf smoke: ok\npresented frame limit: {frame_limit}\nframes presented: {}\ntarget fps: {target_fps}\nmonitor refresh mhz: {monitor_refresh_millihertz}\nsurface present mode: {surface_present_mode}\nwindow physical size: {window_size}\nwindow scale milliscale: {window_scale}\nglyph frame presented: {}\nglyph frame size: {}x{}\nglyph frame glyph quads: {}\nglyph frame background quads: {}\nglyph frame decoration quads: {}\nglyph frame cursor quads: {}\nglyph frame atlas bytes: {}\nglyph frame atlas occupied slots: {}\nframe interval target fps: {}\nframe interval target ns: {frame_interval_target_ns}\nelapsed ns: {}\nframe interval samples: {}\nframe interval avg ns: {}\nframe interval max ns: {}\nframe interval p95 ns: {}\nframe interval p95 exact ns: {}\nframe intervals over target: {}\nframe intervals over double target: {}\ndropped frames: {}\nframe pacing accepted: {}\n",
                        report.frames_presented,
                        report.glyph_frame_presented,
                        report.glyph_frame_width,
                        report.glyph_frame_height,
                        report.glyph_frame_glyph_quads,
                        report.glyph_frame_background_quads,
                        report.glyph_frame_decoration_quads,
                        report.glyph_frame_cursor_quads,
                        report.glyph_frame_atlas_bytes,
                        report.glyph_frame_atlas_occupied_slots,
                        report.frame_interval_target_fps,
                        started_at.elapsed().as_nanos(),
                        report.frame_interval_samples,
                        report.frame_interval_avg_ns,
                        report.frame_interval_max_ns,
                        report.frame_interval_p95_ns,
                        report.frame_interval_p95_exact_ns,
                        report.frame_intervals_over_target,
                        report.frame_intervals_over_double_target,
                        report.dropped_frames,
                        frame_pacing_accepted
                    ),
                    stderr: String::new(),
                }
            } else {
                CliExit {
                    code: 0,
                    stdout: "window smoke: ok\npresented frame limit: 1\n".to_owned(),
                    stderr: String::new(),
                }
            }
        }
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("{error}\n"),
        },
    }
}

fn format_window_size(width: Option<u32>, height: Option<u32>) -> String {
    match (width, height) {
        (Some(width), Some(height)) => format!("{width}x{height}"),
        _ => "unknown".to_owned(),
    }
}

fn window_frame_pacing_accepted(report: &crate::app::NativeAppRunReport) -> bool {
    let target_interval_ns = 1_000_000_000 / u64::from(report.frame_interval_target_fps.max(1));
    let p95_ns = if report.frame_interval_p95_exact_ns > 0 {
        report.frame_interval_p95_exact_ns
    } else {
        report.frame_interval_p95_ns
    };
    report.frame_interval_samples > 0 && p95_ns <= target_interval_ns && report.dropped_frames == 0
}

fn window_smoke_command_name(command: CliCommand<'_>) -> &'static str {
    match command {
        CliCommand::WindowSmoke => "--window-smoke",
        CliCommand::WindowPerfSmoke => "--window-perf-smoke",
        _ => unreachable!(),
    }
}
