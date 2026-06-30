//! CLI command dispatch.

mod argument_commands;
mod arguments;

use super::args::{CliCommand, command_for, usage};
use super::clipboard_smoke::{clipboard_smoke_exit, osc52_clipboard_smoke_exit};
use super::config_commands::{NativeAppLaunchConfig, launch_native_app_exit};
use super::font_smoke::font_symbol_fallback_smoke_exit;
use super::frame_scheduler_smoke::frame_scheduler_smoke_exit;
use super::gpu::{GpuCommandContext, gpu_command_exit};
use super::runtime_alternate_screen_smoke::runtime_alternate_screen_smoke_exit;
use super::runtime_bracketed_paste_smoke::runtime_bracketed_paste_smoke_exit;
use super::runtime_clipboard_smoke::runtime_clipboard_paste_smoke_exit;
use super::runtime_config_reload_smoke::runtime_config_reload_smoke_exit;
use super::runtime_glyph_frame_smoke::runtime_glyph_frame_smoke_exit;
use super::runtime_input_smoke::{
    runtime_committed_text_smoke_exit, runtime_focus_smoke_exit, runtime_idle_cpu_smoke_exit,
    runtime_idle_smoke_exit, runtime_mouse_smoke_exit, runtime_perf_budget_smoke_exit,
    runtime_perf_p95_smoke_exit, runtime_perf_smoke_exit, runtime_response_smoke_exit,
};
use super::runtime_osc52_clipboard_smoke::runtime_osc52_clipboard_smoke_exit;
use super::runtime_output_smoke::{
    runtime_bounded_state_smoke_exit, runtime_continuous_output_smoke_exit,
    runtime_large_output_smoke_exit, runtime_memory_smoke_exit,
};
use super::runtime_real_shell_smoke::{
    runtime_real_shell_command_output_smoke_exit, runtime_real_shell_large_output_smoke_exit,
    runtime_real_shell_perf_budget_smoke_exit, runtime_real_shell_reflow_smoke_exit,
    runtime_real_shell_smoke_exit,
};
use super::runtime_reflow_smoke::runtime_reflow_smoke_exit;
use super::runtime_repaint_smoke::runtime_repaint_smoke_exit;
use super::runtime_scrollback_smoke::runtime_scrollback_smoke_exit;
use super::runtime_selection_copy_smoke::runtime_selection_copy_smoke_exit;
use super::runtime_text_zoom_smoke::runtime_text_zoom_smoke_exit;
use super::runtime_tmux_smoke::runtime_tmux_smoke_exit;
use super::runtime_tool_workflow_smoke::runtime_tool_workflow_smoke_exit;
use super::theme_smoke::{theme_legibility_smoke_exit, theme_list_exit};
use super::tmux_assist::tmux_assist_exit;
use super::tmux_manager::tmux_manager_exit;
use super::{CliExit, NativeAppLauncher};
use crate::clipboard::HostClipboard;
use crate::native_gpu::GpuBootstrapBackend;
use arguments::reject_extra_args;

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
            stderr: format!(
                "{}unknown argument: {arg}\nrun `gromaq --help` for usage\n",
                usage()
            ),
        };
    };
    if let Some(exit) =
        argument_commands::run_argument_command(command, &mut args, backend, app_launcher)
    {
        return exit;
    }
    if let Err(exit) = reject_extra_args(&mut args) {
        return exit;
    }

    match command {
        CliCommand::Help => CliExit {
            code: 0,
            stdout: usage(),
            stderr: String::new(),
        },
        CliCommand::Version => CliExit {
            code: 0,
            stdout: format!("gromaq {}\n", env!("CARGO_PKG_VERSION")),
            stderr: String::new(),
        },
        CliCommand::Gpu(arg) => gpu_command_exit(arg, backend),
        CliCommand::GpuTerminalTextSnapshot => unreachable!(),
        CliCommand::GpuWelcomeImageSnapshot => unreachable!(),
        CliCommand::ClipboardSmoke => clipboard_smoke_exit(clipboard),
        CliCommand::Osc52ClipboardSmoke => osc52_clipboard_smoke_exit(clipboard),
        CliCommand::RuntimeClipboardPasteSmoke => runtime_clipboard_paste_smoke_exit(clipboard),
        CliCommand::RuntimeOsc52ClipboardSmoke => runtime_osc52_clipboard_smoke_exit(clipboard),
        CliCommand::RuntimeBracketedPasteSmoke => runtime_bracketed_paste_smoke_exit(),
        CliCommand::RuntimeSelectionCopySmoke => runtime_selection_copy_smoke_exit(clipboard),
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
        CliCommand::RuntimeRealShellCommandOutputSmoke => {
            runtime_real_shell_command_output_smoke_exit()
        }
        CliCommand::RuntimeRealShellPerfBudgetSmoke => runtime_real_shell_perf_budget_smoke_exit(),
        CliCommand::RuntimeRealShellLargeOutputSmoke => {
            runtime_real_shell_large_output_smoke_exit()
        }
        CliCommand::RuntimeRealShellReflowSmoke => runtime_real_shell_reflow_smoke_exit(),
        CliCommand::RuntimeAlternateScreenSmoke => runtime_alternate_screen_smoke_exit(),
        CliCommand::RuntimeReflowSmoke => runtime_reflow_smoke_exit(),
        CliCommand::RuntimeConfigReloadSmoke => runtime_config_reload_smoke_exit(),
        CliCommand::RuntimeTextZoomSmoke => runtime_text_zoom_smoke_exit(),
        CliCommand::RuntimeRepaintSmoke => runtime_repaint_smoke_exit(),
        CliCommand::RuntimeToolWorkflowSmoke => runtime_tool_workflow_smoke_exit(),
        CliCommand::RuntimeTmuxSmoke => runtime_tmux_smoke_exit(),
        CliCommand::TmuxAssist => tmux_assist_exit(),
        CliCommand::TmuxManager => tmux_manager_exit(),
        CliCommand::TmuxAction => unreachable!(),
        CliCommand::FontSymbolFallbackSmoke => font_symbol_fallback_smoke_exit(),
        CliCommand::ThemeList => theme_list_exit(),
        CliCommand::ThemeLegibilitySmoke => theme_legibility_smoke_exit(),
        CliCommand::ThemePreviewSnapshot => unreachable!(),
        CliCommand::ThemePreviewConfig => unreachable!(),
        CliCommand::WelcomePreviewSnapshot => unreachable!(),
        CliCommand::RuntimeFocusSmoke => runtime_focus_smoke_exit(),
        CliCommand::RuntimeMouseSmoke => runtime_mouse_smoke_exit(),
        CliCommand::RuntimeResponseSmoke => runtime_response_smoke_exit(),
        CliCommand::RuntimeCommittedTextSmoke => runtime_committed_text_smoke_exit(),
        CliCommand::RuntimeIdleSmoke => runtime_idle_smoke_exit(),
        CliCommand::RuntimeIdleCpuSmoke => runtime_idle_cpu_smoke_exit(),
        CliCommand::FrameSchedulerSmoke => frame_scheduler_smoke_exit(),
        CliCommand::Config
        | CliCommand::ConfigCheck
        | CliCommand::ConfigTemplate
        | CliCommand::ThemeExport
        | CliCommand::WindowSmoke
        | CliCommand::WindowPerfSmoke
        | CliCommand::WindowScreenshotSmoke
        | CliCommand::WindowGlyphFrameSnapshot => unreachable!(),
    }
}
