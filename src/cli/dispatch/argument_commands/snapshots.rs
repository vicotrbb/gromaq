//! Snapshot-oriented CLI argument commands.

use crate::cli::dispatch::arguments::{reject_extra_args, required_snapshot_path_arg};
use crate::cli::gpu::{GpuCommandContext, gpu_terminal_text_snapshot_exit};
use crate::cli::runtime_glyph_frame_smoke::runtime_glyph_frame_snapshot_exit;
use crate::cli::theme_smoke::theme_preview_snapshot_exit;
use crate::cli::window_smoke::window_glyph_frame_snapshot_exit;
use crate::cli::{CliExit, NativeAppLauncher};
use crate::native_gpu::GpuBootstrapBackend;

pub(super) fn gpu_terminal_text_snapshot_command<I, S, B>(args: &mut I, backend: &B) -> CliExit
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

pub(super) fn runtime_glyph_frame_snapshot_command<I, S>(args: &mut I) -> CliExit
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

pub(super) fn theme_preview_snapshot_command<I, S>(args: &mut I) -> CliExit
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

pub(super) fn window_glyph_frame_snapshot_command<I, S, A>(
    args: &mut I,
    app_launcher: Option<&A>,
) -> CliExit
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
