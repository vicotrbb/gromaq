//! Command-line entry points for the native application.

mod clipboard_smoke;
mod config_commands;
mod frame_scheduler_smoke;
mod gpu;
mod runtime_alternate_screen_smoke;
mod runtime_clipboard_smoke;
mod runtime_config_reload_smoke;
mod runtime_glyph_frame_smoke;
mod runtime_input_smoke;
mod runtime_output_smoke;
mod runtime_reflow_smoke;
mod runtime_scrollback_smoke;
use clipboard_smoke::{clipboard_smoke_exit, osc52_clipboard_smoke_exit};
pub use config_commands::{
    NativeAppLaunchConfig, NativeAppLaunchError, NativeAppLauncher, RealNativeAppLauncher,
};
use config_commands::{
    config_check_exit, config_template_exit, launch_config_file_exit, launch_native_app_exit,
};
use frame_scheduler_smoke::frame_scheduler_smoke_exit;
use gpu::gpu_command_exit;
pub use gpu::{AdapterReport, GpuCommandContext};
use runtime_alternate_screen_smoke::runtime_alternate_screen_smoke_exit;
use runtime_clipboard_smoke::runtime_clipboard_paste_smoke_exit;
use runtime_config_reload_smoke::runtime_config_reload_smoke_exit;
use runtime_glyph_frame_smoke::runtime_glyph_frame_smoke_exit;
use runtime_input_smoke::{
    runtime_focus_smoke_exit, runtime_idle_cpu_smoke_exit, runtime_idle_smoke_exit,
    runtime_mouse_smoke_exit, runtime_perf_budget_smoke_exit, runtime_perf_smoke_exit,
    runtime_response_smoke_exit,
};
use runtime_output_smoke::{
    runtime_bounded_state_smoke_exit, runtime_continuous_output_smoke_exit,
    runtime_large_output_smoke_exit, runtime_memory_smoke_exit,
};
use runtime_reflow_smoke::runtime_reflow_smoke_exit;
use runtime_scrollback_smoke::runtime_scrollback_smoke_exit;

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

fn run_with_optional_app_and_clipboard<I, S, B, A, C>(
    args: I,
    backend: &B,
    app_launcher: Option<&A>,
    clipboard: &mut C,
) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    B: GpuBootstrapBackend,
    B::Context: GpuCommandContext,
    A: NativeAppLauncher,
    C: HostClipboard,
{
    let mut args = args.into_iter();
    let _program = args.next();
    let Some(arg) = args.next() else {
        if let Some(app_launcher) = app_launcher {
            return launch_native_app_exit(app_launcher, NativeAppLaunchConfig::default());
        }
        return CliExit {
            code: 0,
            stdout: usage(),
            stderr: String::new(),
        };
    };
    let arg = arg.as_ref();
    if arg != "--gpu-info"
        && arg != "--gpu-smoke"
        && arg != "--gpu-upload-smoke"
        && arg != "--gpu-glyph-atlas-smoke"
        && arg != "--gpu-text-atlas-smoke"
        && arg != "--gpu-textured-quad-smoke"
        && arg != "--gpu-terminal-text-smoke"
        && arg != "--clipboard-smoke"
        && arg != "--config"
        && arg != "--config-check"
        && arg != "--config-template"
        && arg != "--osc52-clipboard-smoke"
        && arg != "--runtime-clipboard-paste-smoke"
        && arg != "--runtime-glyph-frame-smoke"
        && arg != "--runtime-scrollback-smoke"
        && arg != "--runtime-perf-smoke"
        && arg != "--runtime-perf-budget-smoke"
        && arg != "--runtime-large-output-smoke"
        && arg != "--runtime-bounded-state-smoke"
        && arg != "--runtime-memory-smoke"
        && arg != "--runtime-continuous-output-smoke"
        && arg != "--runtime-alternate-screen-smoke"
        && arg != "--runtime-reflow-smoke"
        && arg != "--runtime-config-reload-smoke"
        && arg != "--runtime-focus-smoke"
        && arg != "--runtime-mouse-smoke"
        && arg != "--runtime-response-smoke"
        && arg != "--runtime-idle-smoke"
        && arg != "--runtime-idle-cpu-smoke"
        && arg != "--frame-scheduler-smoke"
    {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!("{}unknown argument: {arg}\n", usage()),
        };
    }
    if arg == "--config-check" {
        let Some(path) = args.next() else {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}missing config path for --config-check\n", usage()),
            };
        };
        if let Some(extra) = args.next() {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}unexpected extra argument: {}\n", usage(), extra.as_ref()),
            };
        }
        return config_check_exit(path.as_ref());
    }
    if arg == "--config-template" {
        if let Some(extra) = args.next() {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}unexpected extra argument: {}\n", usage(), extra.as_ref()),
            };
        }
        return config_template_exit();
    }
    if arg == "--config" {
        let Some(path) = args.next() else {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}missing config path for --config\n", usage()),
            };
        };
        if let Some(extra) = args.next() {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}unexpected extra argument: {}\n", usage(), extra.as_ref()),
            };
        }
        let Some(app_launcher) = app_launcher else {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}native app launch unavailable for --config\n", usage()),
            };
        };
        return launch_config_file_exit(path.as_ref(), app_launcher);
    }
    if let Some(extra) = args.next() {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!("{}unexpected extra argument: {}\n", usage(), extra.as_ref(),),
        };
    }

    if arg == "--clipboard-smoke" {
        return clipboard_smoke_exit(clipboard);
    }
    if arg == "--osc52-clipboard-smoke" {
        return osc52_clipboard_smoke_exit(clipboard);
    }
    if arg == "--runtime-clipboard-paste-smoke" {
        return runtime_clipboard_paste_smoke_exit(clipboard);
    }
    if arg == "--runtime-glyph-frame-smoke" {
        return runtime_glyph_frame_smoke_exit();
    }
    if arg == "--runtime-scrollback-smoke" {
        return runtime_scrollback_smoke_exit();
    }
    if arg == "--runtime-perf-smoke" {
        return runtime_perf_smoke_exit();
    }
    if arg == "--runtime-perf-budget-smoke" {
        return runtime_perf_budget_smoke_exit();
    }
    if arg == "--runtime-large-output-smoke" {
        return runtime_large_output_smoke_exit();
    }
    if arg == "--runtime-bounded-state-smoke" {
        return runtime_bounded_state_smoke_exit();
    }
    if arg == "--runtime-memory-smoke" {
        return runtime_memory_smoke_exit();
    }
    if arg == "--runtime-continuous-output-smoke" {
        return runtime_continuous_output_smoke_exit();
    }
    if arg == "--runtime-alternate-screen-smoke" {
        return runtime_alternate_screen_smoke_exit();
    }
    if arg == "--runtime-reflow-smoke" {
        return runtime_reflow_smoke_exit();
    }
    if arg == "--runtime-config-reload-smoke" {
        return runtime_config_reload_smoke_exit();
    }
    if arg == "--runtime-focus-smoke" {
        return runtime_focus_smoke_exit();
    }
    if arg == "--runtime-mouse-smoke" {
        return runtime_mouse_smoke_exit();
    }
    if arg == "--runtime-response-smoke" {
        return runtime_response_smoke_exit();
    }
    if arg == "--runtime-idle-smoke" {
        return runtime_idle_smoke_exit();
    }
    if arg == "--runtime-idle-cpu-smoke" {
        return runtime_idle_cpu_smoke_exit();
    }
    if arg == "--frame-scheduler-smoke" {
        return frame_scheduler_smoke_exit();
    }

    gpu_command_exit(arg, backend)
}

fn usage() -> String {
    "usage: gromaq [--gpu-info|--gpu-smoke|--gpu-upload-smoke|--gpu-glyph-atlas-smoke|--gpu-text-atlas-smoke|--gpu-textured-quad-smoke|--gpu-terminal-text-smoke|--clipboard-smoke|--config <path>|--config-check <path>|--config-template|--osc52-clipboard-smoke|--runtime-clipboard-paste-smoke|--runtime-glyph-frame-smoke|--runtime-scrollback-smoke|--runtime-perf-smoke|--runtime-perf-budget-smoke|--runtime-large-output-smoke|--runtime-bounded-state-smoke|--runtime-memory-smoke|--runtime-continuous-output-smoke|--runtime-alternate-screen-smoke|--runtime-reflow-smoke|--runtime-config-reload-smoke|--runtime-focus-smoke|--runtime-mouse-smoke|--runtime-response-smoke|--runtime-idle-smoke|--runtime-idle-cpu-smoke|--frame-scheduler-smoke]\n".to_owned()
}
