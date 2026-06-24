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
                let frame_pacing_accepted = window_frame_pacing_accepted(&report, target_fps);
                CliExit {
                    code: 0,
                    stdout: format!(
                        "window perf smoke: ok\npresented frame limit: {frame_limit}\nframes presented: {}\ntarget fps: {target_fps}\nelapsed ns: {}\nframe interval samples: {}\nframe interval avg ns: {}\nframe interval max ns: {}\nframe interval p95 ns: {}\ndropped frames: {}\nframe pacing accepted: {}\n",
                        report.frames_presented,
                        started_at.elapsed().as_nanos(),
                        report.frame_interval_samples,
                        report.frame_interval_avg_ns,
                        report.frame_interval_max_ns,
                        report.frame_interval_p95_ns,
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

fn window_frame_pacing_accepted(report: &crate::app::NativeAppRunReport, target_fps: u32) -> bool {
    let target_interval_ns = 1_000_000_000 / u64::from(target_fps.max(1));
    report.frame_interval_samples > 0
        && report.frame_interval_p95_ns <= target_interval_ns
        && report.dropped_frames == 0
}

fn window_smoke_command_name(command: CliCommand<'_>) -> &'static str {
    match command {
        CliCommand::WindowSmoke => "--window-smoke",
        CliCommand::WindowPerfSmoke => "--window-perf-smoke",
        _ => unreachable!(),
    }
}
