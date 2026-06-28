//! Command-line entry points for the native application.

mod args;
mod clipboard_smoke;
mod config_commands;
mod dispatch;
mod font_smoke;
mod frame_scheduler_smoke;
mod gpu;
mod runtime_alternate_screen_smoke;
mod runtime_bracketed_paste_smoke;
mod runtime_clipboard_smoke;
mod runtime_config_reload_smoke;
mod runtime_glyph_frame_smoke;
mod runtime_input_smoke;
mod runtime_osc52_clipboard_smoke;
mod runtime_output_smoke;
mod runtime_real_shell_smoke;
mod runtime_reflow_smoke;
mod runtime_repaint_smoke;
mod runtime_scrollback_smoke;
mod runtime_selection_copy_smoke;
mod runtime_text_zoom_smoke;
mod runtime_tool_workflow_smoke;
mod theme_smoke;
mod window_smoke;
pub use config_commands::{
    NativeAppLaunchConfig, NativeAppLaunchError, NativeAppLauncher, RealNativeAppLauncher,
};
use dispatch::run_with_optional_app_and_clipboard;
pub use gpu::{AdapterReport, GpuCommandContext};

use crate::clipboard::{HostClipboard, NativeClipboard};
use crate::native_gpu::GpuBootstrapBackend;

/// Captured CLI result for tests and the binary wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliExit {
    /// Process exit code.
    pub code: i32,
    /// Standard output text.
    pub stdout: String,
    /// Standard error text.
    pub stderr: String,
}

/// Run the CLI with an injected GPU backend.
pub fn run_with_backend<I, S, B>(args: I, backend: &B) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    B: GpuBootstrapBackend,
    B::Context: GpuCommandContext,
{
    let mut clipboard = NativeClipboard::new();
    run_with_backend_and_clipboard(args, backend, &mut clipboard)
}

/// Run the CLI with injected GPU and clipboard boundaries.
pub fn run_with_backend_and_clipboard<I, S, B, C>(
    args: I,
    backend: &B,
    clipboard: &mut C,
) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    B: GpuBootstrapBackend,
    B::Context: GpuCommandContext,
    C: HostClipboard,
{
    run_with_optional_app_and_clipboard(
        args,
        backend,
        Option::<&RealNativeAppLauncher>::None,
        clipboard,
    )
}

/// Run the CLI with injected GPU and native app launch boundaries.
pub fn run_with_backend_and_app<I, S, B, A>(args: I, backend: &B, app_launcher: &A) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    B: GpuBootstrapBackend,
    B::Context: GpuCommandContext,
    A: NativeAppLauncher,
{
    let mut clipboard = NativeClipboard::new();
    run_with_optional_app_and_clipboard(args, backend, Some(app_launcher), &mut clipboard)
}
