mod config;
mod snapshots;

use super::arguments::reject_extra_args;
use crate::cli::args::{CliCommand, usage};
use crate::cli::gpu::GpuCommandContext;
use crate::cli::theme_smoke::{theme_export_exit, theme_preview_config_exit};
use crate::cli::window_smoke::window_smoke_exit;
use crate::cli::{CliExit, NativeAppLauncher};
use crate::native_gpu::GpuBootstrapBackend;
use config::{config_check_command, config_file_command, config_template_command};
use snapshots::{
    gpu_terminal_text_snapshot_command, gpu_welcome_image_snapshot_command,
    runtime_glyph_frame_snapshot_command, theme_preview_snapshot_command,
    welcome_preview_snapshot_command, window_glyph_frame_snapshot_command,
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
        CliCommand::GpuWelcomeImageSnapshot => {
            Some(gpu_welcome_image_snapshot_command(args, backend))
        }
        CliCommand::RuntimeGlyphFrameSnapshot => Some(runtime_glyph_frame_snapshot_command(args)),
        CliCommand::ThemePreviewSnapshot => Some(theme_preview_snapshot_command(args)),
        CliCommand::WelcomePreviewSnapshot => Some(welcome_preview_snapshot_command(args)),
        CliCommand::ThemePreviewConfig => Some(theme_preview_config_command(args)),
        CliCommand::ThemeExport => Some(theme_export_command(args)),
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

fn theme_preview_config_command<I, S>(args: &mut I) -> CliExit
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let Some(config_path) = args.next() else {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!(
                "{}missing config path for --theme-preview-config\n",
                usage()
            ),
        };
    };
    let Some(snapshot_path) = args.next() else {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!(
                "{}missing snapshot path for --theme-preview-config\n",
                usage()
            ),
        };
    };
    if let Err(exit) = reject_extra_args(args) {
        return exit;
    }
    theme_preview_config_exit(config_path.as_ref(), snapshot_path.as_ref())
}

fn theme_export_command<I, S>(args: &mut I) -> CliExit
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let Some(preset) = args.next() else {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!("{}missing theme preset for --theme-export\n", usage()),
        };
    };
    let Some(path) = args.next() else {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!("{}missing export path for --theme-export\n", usage()),
        };
    };
    if let Err(exit) = reject_extra_args(args) {
        return exit;
    }
    theme_export_exit(preset.as_ref(), path.as_ref())
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
