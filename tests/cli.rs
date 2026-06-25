use std::cell::RefCell;
use std::sync::{Mutex, OnceLock};

use gromaq::HostClipboard;
use gromaq::cli::{
    CliExit, GpuCommandContext, NativeAppLauncher, run_with_backend as gromaq_run_with_backend,
    run_with_backend_and_app as gromaq_run_with_backend_and_app,
    run_with_backend_and_clipboard as gromaq_run_with_backend_and_clipboard,
};
use gromaq::native_gpu::GpuBootstrapBackend;

#[path = "cli/clipboard.rs"]
mod clipboard;
#[path = "cli/config/mod.rs"]
mod config;
#[path = "cli/gpu_smoke.rs"]
mod gpu_smoke;
#[path = "cli/real_shell.rs"]
mod real_shell;
#[path = "cli/runtime_smoke.rs"]
mod runtime_smoke;
#[path = "cli/support.rs"]
mod support;
#[path = "cli/theme.rs"]
mod theme;
#[path = "cli/window/mod.rs"]
mod window;

pub(crate) use support::{
    MockAppLauncher, MockBackend, ReadOnlyClipboard, system_mono_font_path, test_cli_config_path,
};

fn run_with_backend<I, S, B>(args: I, backend: &B) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    B: GpuBootstrapBackend,
    B::Context: GpuCommandContext,
{
    let _guard = cli_invocation_guard();
    gromaq_run_with_backend(args, backend)
}

fn run_with_backend_and_clipboard<I, S, B, C>(args: I, backend: &B, clipboard: &mut C) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    B: GpuBootstrapBackend,
    B::Context: GpuCommandContext,
    C: HostClipboard,
{
    let _guard = cli_invocation_guard();
    gromaq_run_with_backend_and_clipboard(args, backend, clipboard)
}

fn run_with_backend_and_app<I, S, B, A>(args: I, backend: &B, app_launcher: &A) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    B: GpuBootstrapBackend,
    B::Context: GpuCommandContext,
    A: NativeAppLauncher,
{
    let _guard = cli_invocation_guard();
    gromaq_run_with_backend_and_app(args, backend, app_launcher)
}

fn cli_invocation_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn gpu_info_cli_reports_adapter_metadata() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-info"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("Mock GPU"));
    assert!(exit.stdout.contains("MockBackend"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}

#[test]
fn unknown_cli_argument_returns_usage_error() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--wat"], &backend);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.starts_with("usage: gromaq ["));
    assert!(
        exit.stderr
            .contains("--runtime-real-shell-command-output-smoke")
    );
    assert!(exit.stderr.ends_with("unknown argument: --wat\n"));
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn frame_scheduler_smoke_cli_reports_144hz_timeline_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--frame-scheduler-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("frame scheduler smoke: ok"));
    assert!(exit.stdout.contains("target fps: 144"));
    assert!(exit.stdout.contains("target interval ns: 6944444"));
    assert!(exit.stdout.contains("frame-paced wait ns:"));
    assert!(exit.stdout.contains("frames presented: 3"));
    assert!(exit.stdout.contains("dropped frames: 2"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_real_shell_command_output_smoke_preserves_output_after_prompt_redraw() {
    let _guard = real_shell::real_shell_test_guard();
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(
        ["gromaq", "--runtime-real-shell-command-output-smoke"],
        &backend,
    );

    assert_eq!(exit.code, 0);
    assert!(
        exit.stdout
            .contains("runtime real-shell command-output smoke: ok")
    );
    assert!(exit.stdout.contains("shell: /bin/sh"));
    assert!(exit.stdout.contains("command output observed: true"));
    assert!(exit.stdout.contains("prompt observed: true"));
    assert!(exit.stdout.contains("full redraw preserved output: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_text_zoom_smoke_reports_browser_style_zoom_metrics_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-text-zoom-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime text zoom smoke: ok"));
    assert!(exit.stdout.contains("default font size px: 34"));
    assert!(exit.stdout.contains("default cell width px: 19"));
    assert!(exit.stdout.contains("default line height px: 47"));
    assert!(exit.stdout.contains("zoomed font size px: 39"));
    assert!(exit.stdout.contains("zoomed cell width px: 22"));
    assert!(exit.stdout.contains("zoomed line height px: 54"));
    assert!(exit.stdout.contains("zoom in reduced grid: true"));
    assert!(exit.stdout.contains("reset restored metrics: true"));
    assert!(exit.stdout.contains("reset restored grid: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}
