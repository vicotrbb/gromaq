use super::arguments::{reject_extra_args, required_path_arg, required_snapshot_path_arg};
use crate::cli::args::{CliCommand, usage};
use crate::cli::config_commands::{
    config_check_exit, config_template_exit, launch_config_file_exit,
};
use crate::cli::gpu::{GpuCommandContext, gpu_terminal_text_snapshot_exit};
use crate::cli::runtime_glyph_frame_smoke::runtime_glyph_frame_snapshot_exit;
use crate::cli::theme_smoke::theme_preview_snapshot_exit;
use crate::cli::window_smoke::{window_glyph_frame_snapshot_exit, window_smoke_exit};
use crate::cli::{CliExit, NativeAppLauncher};
use crate::native_gpu::GpuBootstrapBackend;

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

fn config_check_command<I, S>(args: &mut I) -> CliExit
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let path = match required_path_arg(args, "--config-check") {
        Ok(path) => path,
        Err(exit) => return exit,
    };
    if let Err(exit) = reject_extra_args(args) {
        return exit;
    }
    config_check_exit(path.as_ref())
}

fn config_template_command<I, S>(args: &mut I) -> CliExit
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    if let Err(exit) = reject_extra_args(args) {
        return exit;
    }
    config_template_exit()
}

fn gpu_terminal_text_snapshot_command<I, S, B>(args: &mut I, backend: &B) -> CliExit
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
    B: GpuBootstrapBackend,
    B::Context: GpuCommandContext,
{
    let path = match required_snapshot_path_arg(args, "--gpu-terminal-text-snapshot") {
        Ok(path) => path,
        Err(exit) => return exit,
    };
    if let Err(exit) = reject_extra_args(args) {
        return exit;
    }
    gpu_terminal_text_snapshot_exit(path.as_ref(), backend)
}

fn runtime_glyph_frame_snapshot_command<I, S>(args: &mut I) -> CliExit
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let path = match required_snapshot_path_arg(args, "--runtime-glyph-frame-snapshot") {
        Ok(path) => path,
        Err(exit) => return exit,
    };
    if let Err(exit) = reject_extra_args(args) {
        return exit;
    }
    runtime_glyph_frame_snapshot_exit(path.as_ref())
}

fn theme_preview_snapshot_command<I, S>(args: &mut I) -> CliExit
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let path = match required_snapshot_path_arg(args, "--theme-preview-snapshot") {
        Ok(path) => path,
        Err(exit) => return exit,
    };
    if let Err(exit) = reject_extra_args(args) {
        return exit;
    }
    theme_preview_snapshot_exit(path.as_ref())
}

fn window_glyph_frame_snapshot_command<I, S, A>(args: &mut I, app_launcher: Option<&A>) -> CliExit
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
    A: NativeAppLauncher,
{
    let path = match required_snapshot_path_arg(args, "--window-glyph-frame-snapshot") {
        Ok(path) => path,
        Err(exit) => return exit,
    };
    if let Err(exit) = reject_extra_args(args) {
        return exit;
    }
    window_glyph_frame_snapshot_exit(path.as_ref(), app_launcher)
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

fn config_file_command<I, S, A>(args: &mut I, app_launcher: Option<&A>) -> CliExit
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
    A: NativeAppLauncher,
{
    let path = match required_path_arg(args, "--config") {
        Ok(path) => path,
        Err(exit) => return exit,
    };
    if let Err(exit) = reject_extra_args(args) {
        return exit;
    }
    let Some(app_launcher) = app_launcher else {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!("{}native app launch unavailable for --config\n", usage()),
        };
    };
    launch_config_file_exit(path.as_ref(), app_launcher)
}
