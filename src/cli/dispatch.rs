//! CLI command dispatch.

use super::args::{CliCommand, command_for, usage};
use super::clipboard_smoke::{clipboard_smoke_exit, osc52_clipboard_smoke_exit};
use super::config_commands::{
    NativeAppLaunchConfig, config_check_exit, config_template_exit, launch_config_file_exit,
    launch_native_app_exit,
};
use super::frame_scheduler_smoke::frame_scheduler_smoke_exit;
use super::gpu::{GpuCommandContext, gpu_command_exit, gpu_terminal_text_snapshot_exit};
use super::runtime_alternate_screen_smoke::runtime_alternate_screen_smoke_exit;
use super::runtime_clipboard_smoke::runtime_clipboard_paste_smoke_exit;
use super::runtime_config_reload_smoke::runtime_config_reload_smoke_exit;
use super::runtime_glyph_frame_smoke::{
    runtime_glyph_frame_smoke_exit, runtime_glyph_frame_snapshot_exit,
};
use super::runtime_input_smoke::{
    runtime_focus_smoke_exit, runtime_idle_cpu_smoke_exit, runtime_idle_smoke_exit,
    runtime_mouse_smoke_exit, runtime_perf_budget_smoke_exit, runtime_perf_p95_smoke_exit,
    runtime_perf_smoke_exit, runtime_response_smoke_exit,
};
use super::runtime_output_smoke::{
    runtime_bounded_state_smoke_exit, runtime_continuous_output_smoke_exit,
    runtime_large_output_smoke_exit, runtime_memory_smoke_exit,
};
use super::runtime_real_shell_smoke::{
    runtime_real_shell_large_output_smoke_exit, runtime_real_shell_reflow_smoke_exit,
    runtime_real_shell_smoke_exit,
};
use super::runtime_reflow_smoke::runtime_reflow_smoke_exit;
use super::runtime_scrollback_smoke::runtime_scrollback_smoke_exit;
use super::window_smoke::window_smoke_exit;
use super::{CliExit, NativeAppLauncher};
use crate::clipboard::HostClipboard;
use crate::native_gpu::GpuBootstrapBackend;

pub(super) fn run_with_optional_app_and_clipboard<I, S, B, A, C>(
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
    if command == CliCommand::GpuTerminalTextSnapshot {
        let Some(path) = args.next() else {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!(
                    "{}missing snapshot path for --gpu-terminal-text-snapshot\n",
                    usage()
                ),
            };
        };
        if let Some(extra) = args.next() {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}unexpected extra argument: {}\n", usage(), extra.as_ref()),
            };
        }
        return gpu_terminal_text_snapshot_exit(path.as_ref(), backend);
    }
    if command == CliCommand::RuntimeGlyphFrameSnapshot {
        let Some(path) = args.next() else {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!(
                    "{}missing snapshot path for --runtime-glyph-frame-snapshot\n",
                    usage()
                ),
            };
        };
        if let Some(extra) = args.next() {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}unexpected extra argument: {}\n", usage(), extra.as_ref()),
            };
        }
        return runtime_glyph_frame_snapshot_exit(path.as_ref());
    }
    if matches!(
        command,
        CliCommand::WindowSmoke | CliCommand::WindowPerfSmoke
    ) {
        if let Some(extra) = args.next() {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}unexpected extra argument: {}\n", usage(), extra.as_ref()),
            };
        }
        return window_smoke_exit(command, app_launcher);
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
        CliCommand::GpuTerminalTextSnapshot => unreachable!(),
        CliCommand::ClipboardSmoke => clipboard_smoke_exit(clipboard),
        CliCommand::Osc52ClipboardSmoke => osc52_clipboard_smoke_exit(clipboard),
        CliCommand::RuntimeClipboardPasteSmoke => runtime_clipboard_paste_smoke_exit(clipboard),
        CliCommand::RuntimeGlyphFrameSmoke => runtime_glyph_frame_smoke_exit(),
        CliCommand::RuntimeGlyphFrameSnapshot => unreachable!(),
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
