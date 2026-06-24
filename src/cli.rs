//! Command-line entry points for the native application.

use std::time::Instant;

mod args;
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
mod runtime_real_shell_smoke;
mod runtime_reflow_smoke;
mod runtime_scrollback_smoke;
use args::{CliCommand, command_for, usage};
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
    runtime_mouse_smoke_exit, runtime_perf_budget_smoke_exit, runtime_perf_p95_smoke_exit,
    runtime_perf_smoke_exit, runtime_response_smoke_exit,
};
use runtime_output_smoke::{
    runtime_bounded_state_smoke_exit, runtime_continuous_output_smoke_exit,
    runtime_large_output_smoke_exit, runtime_memory_smoke_exit,
};
use runtime_real_shell_smoke::{
    runtime_real_shell_large_output_smoke_exit, runtime_real_shell_reflow_smoke_exit,
    runtime_real_shell_smoke_exit,
};
use runtime_reflow_smoke::runtime_reflow_smoke_exit;
use runtime_scrollback_smoke::runtime_scrollback_smoke_exit;

use crate::clipboard::{HostClipboard, NativeClipboard};
use crate::native_gpu::GpuBootstrapBackend;
use crate::pty::ShellCommand;

const WINDOW_PERF_SMOKE_FRAME_LIMIT: u64 = 16;

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
    let Some(command) = command_for(arg) else {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!("{}unknown argument: {arg}\n", usage()),
        };
    };
    if command == CliCommand::ConfigCheck {
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
    if command == CliCommand::ConfigTemplate {
        if let Some(extra) = args.next() {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}unexpected extra argument: {}\n", usage(), extra.as_ref()),
            };
        }
        return config_template_exit();
    }
    if command == CliCommand::WindowSmoke || command == CliCommand::WindowPerfSmoke {
        if let Some(extra) = args.next() {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}unexpected extra argument: {}\n", usage(), extra.as_ref()),
            };
        }
        let Some(app_launcher) = app_launcher else {
            let command_name = match command {
                CliCommand::WindowSmoke => "--window-smoke",
                CliCommand::WindowPerfSmoke => "--window-perf-smoke",
                _ => unreachable!(),
            };
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!(
                    "{}native app launch unavailable for {command_name}\n",
                    usage(),
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
        return match app_launcher.launch(launch_config) {
            Ok(report) => {
                if command == CliCommand::WindowPerfSmoke {
                    CliExit {
                        code: 0,
                        stdout: format!(
                            "window perf smoke: ok\npresented frame limit: {frame_limit}\nframes presented: {}\ntarget fps: {target_fps}\nelapsed ns: {}\nframe interval samples: {}\nframe interval avg ns: {}\nframe interval max ns: {}\nframe interval p95 ns: {}\n",
                            report.frames_presented,
                            started_at.elapsed().as_nanos(),
                            report.frame_interval_samples,
                            report.frame_interval_avg_ns,
                            report.frame_interval_max_ns,
                            report.frame_interval_p95_ns
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
        };
    }
    if command == CliCommand::Config {
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

    match command {
        CliCommand::Gpu(arg) => gpu_command_exit(arg, backend),
        CliCommand::ClipboardSmoke => clipboard_smoke_exit(clipboard),
        CliCommand::Osc52ClipboardSmoke => osc52_clipboard_smoke_exit(clipboard),
        CliCommand::RuntimeClipboardPasteSmoke => runtime_clipboard_paste_smoke_exit(clipboard),
        CliCommand::RuntimeGlyphFrameSmoke => runtime_glyph_frame_smoke_exit(),
        CliCommand::RuntimeScrollbackSmoke => runtime_scrollback_smoke_exit(),
        CliCommand::RuntimePerfSmoke => runtime_perf_smoke_exit(),
        CliCommand::RuntimePerfBudgetSmoke => runtime_perf_budget_smoke_exit(),
        CliCommand::RuntimePerfP95Smoke => runtime_perf_p95_smoke_exit(),
        CliCommand::RuntimeLargeOutputSmoke => runtime_large_output_smoke_exit(),
        CliCommand::RuntimeBoundedStateSmoke => runtime_bounded_state_smoke_exit(),
        CliCommand::RuntimeMemorySmoke => runtime_memory_smoke_exit(),
        CliCommand::RuntimeContinuousOutputSmoke => runtime_continuous_output_smoke_exit(),
        CliCommand::RuntimeRealShellSmoke => runtime_real_shell_smoke_exit(),
        CliCommand::RuntimeRealShellLargeOutputSmoke => {
            runtime_real_shell_large_output_smoke_exit()
        }
        CliCommand::RuntimeRealShellReflowSmoke => runtime_real_shell_reflow_smoke_exit(),
        CliCommand::RuntimeAlternateScreenSmoke => runtime_alternate_screen_smoke_exit(),
        CliCommand::RuntimeReflowSmoke => runtime_reflow_smoke_exit(),
        CliCommand::RuntimeConfigReloadSmoke => runtime_config_reload_smoke_exit(),
        CliCommand::RuntimeFocusSmoke => runtime_focus_smoke_exit(),
        CliCommand::RuntimeMouseSmoke => runtime_mouse_smoke_exit(),
        CliCommand::RuntimeResponseSmoke => runtime_response_smoke_exit(),
        CliCommand::RuntimeIdleSmoke => runtime_idle_smoke_exit(),
        CliCommand::RuntimeIdleCpuSmoke => runtime_idle_cpu_smoke_exit(),
        CliCommand::FrameSchedulerSmoke => frame_scheduler_smoke_exit(),
        CliCommand::Config
        | CliCommand::ConfigCheck
        | CliCommand::ConfigTemplate
        | CliCommand::WindowSmoke
        | CliCommand::WindowPerfSmoke => unreachable!(),
    }
}
