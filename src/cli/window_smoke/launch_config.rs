use std::path::PathBuf;

use crate::cli::NativeAppLaunchConfig;
use crate::cli::args::CliCommand;
use crate::pty::ShellCommand;

const WINDOW_PERF_SMOKE_FRAME_LIMIT: u64 = 180;
const WINDOW_PERF_SMOKE_WARMUP_FRAMES: u64 = 12;
const WINDOW_SCREENSHOT_SMOKE_FRAME_LIMIT: u64 = 900;
const WINDOW_SMOKE_FRAME_LIMIT: u64 = 3;
const WINDOW_SMOKE_REDRAW_ATTEMPT_LIMIT: u64 = 16;
const WINDOW_PERF_SMOKE_REDRAW_ATTEMPT_MULTIPLIER: u64 = 4;
const WINDOW_SCREENSHOT_SMOKE_REDRAW_ATTEMPT_MULTIPLIER: u64 = 4;

pub(super) fn window_smoke_launch_config(command: CliCommand<'_>) -> (NativeAppLaunchConfig, u64) {
    let mut launch_config = NativeAppLaunchConfig::default();
    let frame_limit = match command {
        CliCommand::WindowSmoke => WINDOW_SMOKE_FRAME_LIMIT,
        CliCommand::WindowPerfSmoke => {
            launch_config.app.frame_interval_warmup_frames = WINDOW_PERF_SMOKE_WARMUP_FRAMES;
            WINDOW_PERF_SMOKE_FRAME_LIMIT + WINDOW_PERF_SMOKE_WARMUP_FRAMES
        }
        CliCommand::WindowScreenshotSmoke => WINDOW_SCREENSHOT_SMOKE_FRAME_LIMIT,
        CliCommand::WindowGlyphFrameSnapshot => unreachable!(),
        _ => unreachable!(),
    };
    launch_config.app.exit_after_presented_frames = Some(frame_limit);
    launch_config.app.exit_after_redraw_attempts = Some(match command {
        CliCommand::WindowSmoke => WINDOW_SMOKE_REDRAW_ATTEMPT_LIMIT,
        CliCommand::WindowPerfSmoke => frame_limit * WINDOW_PERF_SMOKE_REDRAW_ATTEMPT_MULTIPLIER,
        CliCommand::WindowScreenshotSmoke => {
            frame_limit * WINDOW_SCREENSHOT_SMOKE_REDRAW_ATTEMPT_MULTIPLIER
        }
        CliCommand::WindowGlyphFrameSnapshot => unreachable!(),
        _ => unreachable!(),
    });
    launch_config.app.redraw_until_presented_frame_limit = true;
    if command == CliCommand::WindowPerfSmoke {
        launch_config.app.startup_text =
            Some("gromaq window perf smoke\nframe pacing probe\n".to_owned());
        launch_config.runtime.shell = ShellCommand {
            program: "/bin/sh".into(),
            args: vec![
                "-lc".into(),
                "printf 'gromaq window perf smoke\\nframe pacing probe\\n'".into(),
            ],
            cwd: None,
        };
    }
    (launch_config, frame_limit)
}

pub(super) fn window_glyph_frame_snapshot_launch_config(path: &str) -> NativeAppLaunchConfig {
    let mut launch_config = NativeAppLaunchConfig::default();
    configure_snapshot_launch(&mut launch_config, path);
    launch_config.app.startup_text = Some("gromaq window glyph frame snapshot\n".to_owned());
    launch_config.runtime.shell = ShellCommand {
        program: "/bin/sh".into(),
        args: vec![
            "-lc".into(),
            "printf 'gromaq window glyph frame snapshot\\n'; sleep 1".into(),
        ],
        cwd: None,
    };
    launch_config
}

pub(super) fn window_tmux_manager_snapshot_launch_config(path: &str) -> NativeAppLaunchConfig {
    let mut launch_config = NativeAppLaunchConfig::default();
    configure_snapshot_launch(&mut launch_config, path);
    launch_config.app.open_tmux_manager_on_start = true;
    launch_config
}

fn configure_snapshot_launch(launch_config: &mut NativeAppLaunchConfig, path: &str) {
    launch_config.app.exit_after_presented_frames = Some(60);
    launch_config.app.exit_after_redraw_attempts = Some(60);
    launch_config.app.redraw_until_presented_frame_limit = true;
    launch_config.app.glyph_frame_snapshot_path = Some(PathBuf::from(path));
}
