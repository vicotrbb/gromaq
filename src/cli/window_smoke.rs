use std::time::Instant;

use super::args::{CliCommand, usage};
use super::{CliExit, NativeAppLauncher};

mod launch_config;
mod output;

use launch_config::{window_glyph_frame_snapshot_launch_config, window_smoke_launch_config};
use output::{
    window_glyph_frame_snapshot_failure, window_glyph_frame_snapshot_success,
    window_perf_no_glyph_failure, window_perf_success, window_screenshot_smoke_success,
    window_smoke_no_surface_failure, window_smoke_success,
};

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

    let (launch_config, frame_limit) = window_smoke_launch_config(command);
    let target_fps = launch_config.app.target_fps;
    let started_at = Instant::now();
    match app_launcher.launch(launch_config) {
        Ok(report) => {
            if command == CliCommand::WindowPerfSmoke {
                if !report.glyph_frame_presented {
                    return window_perf_no_glyph_failure(&report);
                }
                window_perf_success(&report, frame_limit, target_fps, started_at.elapsed())
            } else if command == CliCommand::WindowScreenshotSmoke {
                if report.frames_presented == 0 {
                    window_smoke_no_surface_failure(&report)
                } else {
                    window_screenshot_smoke_success(&report, frame_limit)
                }
            } else {
                if report.frames_presented == 0 {
                    window_smoke_no_surface_failure(&report)
                } else {
                    window_smoke_success(&report)
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

pub(super) fn window_glyph_frame_snapshot_exit<A>(path: &str, app_launcher: Option<&A>) -> CliExit
where
    A: NativeAppLauncher,
{
    let Some(app_launcher) = app_launcher else {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!(
                "{}native app launch unavailable for --window-glyph-frame-snapshot\n",
                usage()
            ),
        };
    };

    let launch_config = window_glyph_frame_snapshot_launch_config(path);
    match app_launcher.launch(launch_config) {
        Ok(report) if report.glyph_frame_snapshot_written => {
            window_glyph_frame_snapshot_success(path, &report)
        }
        Ok(report) => window_glyph_frame_snapshot_failure(&report),
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("{error}\n"),
        },
    }
}

fn window_smoke_command_name(command: CliCommand<'_>) -> &'static str {
    match command {
        CliCommand::WindowSmoke => "--window-smoke",
        CliCommand::WindowPerfSmoke => "--window-perf-smoke",
        CliCommand::WindowScreenshotSmoke => "--window-screenshot-smoke",
        CliCommand::WindowGlyphFrameSnapshot => "--window-glyph-frame-snapshot",
        _ => unreachable!(),
    }
}
