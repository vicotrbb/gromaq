mod config;
mod snapshots;

use super::arguments::reject_extra_args;
use crate::cli::args::CliCommand;
use crate::cli::gpu::GpuCommandContext;
use crate::cli::window_smoke::window_smoke_exit;
use crate::cli::{CliExit, NativeAppLauncher};
use crate::native_gpu::GpuBootstrapBackend;
use config::{config_check_command, config_file_command, config_template_command};
use snapshots::{
    gpu_terminal_text_snapshot_command, runtime_glyph_frame_snapshot_command,
    theme_preview_snapshot_command, window_glyph_frame_snapshot_command,
};

pub(super) fn run_argument_command<I, S, B, A>(
    command: CliCommand<'_>,
    args: &mut I,
    backend: &B,
    app_launcher: Option<&A>,
) -> Option<CliExit>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
    B: GpuBootstrapBackend,
    B::Context: GpuCommandContext,
    A: NativeAppLauncher,
{
    match command {
        CliCommand::ConfigCheck => Some(config_check_command(args)),
        CliCommand::ConfigTemplate => Some(config_template_command(args)),
        CliCommand::GpuTerminalTextSnapshot => {
            Some(gpu_terminal_text_snapshot_command(args, backend))
        }
        CliCommand::RuntimeGlyphFrameSnapshot => Some(runtime_glyph_frame_snapshot_command(args)),
        CliCommand::ThemePreviewSnapshot => Some(theme_preview_snapshot_command(args)),
        CliCommand::WindowGlyphFrameSnapshot => {
            Some(window_glyph_frame_snapshot_command(args, app_launcher))
        }
        CliCommand::WindowSmoke | CliCommand::WindowPerfSmoke => {
            Some(window_smoke_command(command, args, app_launcher))
        }
        CliCommand::Config => Some(config_file_command(args, app_launcher)),
        _ => None,
    }
}

fn window_smoke_command<I, S, A>(
    command: CliCommand<'_>,
    args: &mut I,
    app_launcher: Option<&A>,
) -> CliExit
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
    A: NativeAppLauncher,
{
    if let Err(exit) = reject_extra_args(args) {
        return exit;
    }
    window_smoke_exit(command, app_launcher)
}
