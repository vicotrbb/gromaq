//! CLI argument vocabulary and usage text.

mod usage;

pub(super) use usage::usage;

/// Parsed top-level command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CliCommand<'a> {
    /// GPU-backed smoke or diagnostics command.
    Gpu(&'a str),
    /// GPU terminal text snapshot export command.
    GpuTerminalTextSnapshot,
    /// GPU welcome splash avatar image snapshot export command.
    GpuWelcomeImageSnapshot,
    /// Host clipboard smoke command.
    ClipboardSmoke,
    /// Config-file native launch command.
    Config,
    /// Config-file validation command.
    ConfigCheck,
    /// Starter config template command.
    ConfigTemplate,
    /// Bounded native window launch smoke command.
    WindowSmoke,
    /// Bounded native window multi-frame timing smoke command.
    WindowPerfSmoke,
    /// Bounded native window host command for desktop screenshot capture.
    WindowScreenshotSmoke,
    /// Bounded native window glyph-frame snapshot export command.
    WindowGlyphFrameSnapshot,
    /// OSC 52 clipboard smoke command.
    Osc52ClipboardSmoke,
    /// Runtime clipboard paste smoke command.
    RuntimeClipboardPasteSmoke,
    /// Runtime glyph-frame smoke command.
    RuntimeGlyphFrameSmoke,
    /// Runtime glyph-frame snapshot export command.
    RuntimeGlyphFrameSnapshot,
    /// Runtime local scrollback smoke command.
    RuntimeScrollbackSmoke,
    /// Runtime performance plumbing smoke command.
    RuntimePerfSmoke,
    /// Runtime performance budget smoke command.
    RuntimePerfBudgetSmoke,
    /// Repeated runtime p95 performance smoke command.
    RuntimePerfP95Smoke,
    /// Runtime large-output smoke command.
    RuntimeLargeOutputSmoke,
    /// Runtime bounded-state smoke command.
    RuntimeBoundedStateSmoke,
    /// Runtime memory smoke command.
    RuntimeMemorySmoke,
    /// Runtime continuous-output smoke command.
    RuntimeContinuousOutputSmoke,
    /// Runtime real shell PTY smoke command.
    RuntimeRealShellSmoke,
    /// Runtime real shell command-output preservation smoke command.
    RuntimeRealShellCommandOutputSmoke,
    /// Runtime real shell performance-budget smoke command.
    RuntimeRealShellPerfBudgetSmoke,
    /// Runtime real shell large-output smoke command.
    RuntimeRealShellLargeOutputSmoke,
    /// Runtime real shell resize/reflow smoke command.
    RuntimeRealShellReflowSmoke,
    /// Runtime alternate-screen smoke command.
    RuntimeAlternateScreenSmoke,
    /// Runtime reflow smoke command.
    RuntimeReflowSmoke,
    /// Runtime config-reload smoke command.
    RuntimeConfigReloadSmoke,
    /// Runtime text zoom smoke command.
    RuntimeTextZoomSmoke,
    /// Runtime full-viewport repaint smoke command.
    RuntimeRepaintSmoke,
    /// Runtime real PTY external-tool workflow smoke command.
    RuntimeToolWorkflowSmoke,
    /// Default theme legibility smoke command.
    ThemeLegibilitySmoke,
    /// Built-in theme preset listing command.
    ThemeList,
    /// Built-in theme preset TOML export command.
    ThemeExport,
    /// Default theme rendered preview snapshot command.
    ThemePreviewSnapshot,
    /// Config-file theme rendered preview snapshot command.
    ThemePreviewConfig,
    /// Default welcome screen rendered preview snapshot command.
    WelcomePreviewSnapshot,
    /// Runtime focus smoke command.
    RuntimeFocusSmoke,
    /// Runtime mouse smoke command.
    RuntimeMouseSmoke,
    /// Runtime terminal response smoke command.
    RuntimeResponseSmoke,
    /// Runtime idle smoke command.
    RuntimeIdleSmoke,
    /// Runtime idle CPU smoke command.
    RuntimeIdleCpuSmoke,
    /// Deterministic frame scheduler smoke command.
    FrameSchedulerSmoke,
}

/// Parse one top-level CLI argument.
pub(super) fn command_for(arg: &str) -> Option<CliCommand<'_>> {
    match arg {
        "--gpu-info"
        | "--gpu-smoke"
        | "--gpu-upload-smoke"
        | "--gpu-glyph-atlas-smoke"
        | "--gpu-text-atlas-smoke"
        | "--gpu-textured-quad-smoke"
        | "--gpu-terminal-text-smoke"
        | "--gpu-terminal-text-perf-smoke" => Some(CliCommand::Gpu(arg)),
        "--gpu-terminal-text-snapshot" => Some(CliCommand::GpuTerminalTextSnapshot),
        "--welcome-image-snapshot" => Some(CliCommand::GpuWelcomeImageSnapshot),
        "--clipboard-smoke" => Some(CliCommand::ClipboardSmoke),
        "--config" => Some(CliCommand::Config),
        "--config-check" => Some(CliCommand::ConfigCheck),
        "--config-template" => Some(CliCommand::ConfigTemplate),
        "--window-smoke" => Some(CliCommand::WindowSmoke),
        "--window-perf-smoke" => Some(CliCommand::WindowPerfSmoke),
        "--window-screenshot-smoke" => Some(CliCommand::WindowScreenshotSmoke),
        "--window-glyph-frame-snapshot" => Some(CliCommand::WindowGlyphFrameSnapshot),
        "--osc52-clipboard-smoke" => Some(CliCommand::Osc52ClipboardSmoke),
        "--runtime-clipboard-paste-smoke" => Some(CliCommand::RuntimeClipboardPasteSmoke),
        "--runtime-glyph-frame-smoke" => Some(CliCommand::RuntimeGlyphFrameSmoke),
        "--runtime-glyph-frame-snapshot" => Some(CliCommand::RuntimeGlyphFrameSnapshot),
        "--runtime-scrollback-smoke" => Some(CliCommand::RuntimeScrollbackSmoke),
        "--runtime-perf-smoke" => Some(CliCommand::RuntimePerfSmoke),
        "--runtime-perf-budget-smoke" => Some(CliCommand::RuntimePerfBudgetSmoke),
        "--runtime-perf-p95-smoke" => Some(CliCommand::RuntimePerfP95Smoke),
        "--runtime-large-output-smoke" => Some(CliCommand::RuntimeLargeOutputSmoke),
        "--runtime-bounded-state-smoke" => Some(CliCommand::RuntimeBoundedStateSmoke),
        "--runtime-memory-smoke" => Some(CliCommand::RuntimeMemorySmoke),
        "--runtime-continuous-output-smoke" => Some(CliCommand::RuntimeContinuousOutputSmoke),
        "--runtime-real-shell-smoke" => Some(CliCommand::RuntimeRealShellSmoke),
        "--runtime-real-shell-command-output-smoke" => {
            Some(CliCommand::RuntimeRealShellCommandOutputSmoke)
        }
        "--runtime-real-shell-perf-budget-smoke" => {
            Some(CliCommand::RuntimeRealShellPerfBudgetSmoke)
        }
        "--runtime-real-shell-large-output-smoke" => {
            Some(CliCommand::RuntimeRealShellLargeOutputSmoke)
        }
        "--runtime-real-shell-reflow-smoke" => Some(CliCommand::RuntimeRealShellReflowSmoke),
        "--runtime-alternate-screen-smoke" => Some(CliCommand::RuntimeAlternateScreenSmoke),
        "--runtime-reflow-smoke" => Some(CliCommand::RuntimeReflowSmoke),
        "--runtime-config-reload-smoke" => Some(CliCommand::RuntimeConfigReloadSmoke),
        "--runtime-text-zoom-smoke" => Some(CliCommand::RuntimeTextZoomSmoke),
        "--runtime-repaint-smoke" => Some(CliCommand::RuntimeRepaintSmoke),
        "--runtime-tool-workflow-smoke" => Some(CliCommand::RuntimeToolWorkflowSmoke),
        "--theme-list" => Some(CliCommand::ThemeList),
        "--theme-export" => Some(CliCommand::ThemeExport),
        "--theme-legibility-smoke" => Some(CliCommand::ThemeLegibilitySmoke),
        "--theme-preview-snapshot" => Some(CliCommand::ThemePreviewSnapshot),
        "--theme-preview-config" => Some(CliCommand::ThemePreviewConfig),
        "--welcome-preview-snapshot" => Some(CliCommand::WelcomePreviewSnapshot),
        "--runtime-focus-smoke" => Some(CliCommand::RuntimeFocusSmoke),
        "--runtime-mouse-smoke" => Some(CliCommand::RuntimeMouseSmoke),
        "--runtime-response-smoke" => Some(CliCommand::RuntimeResponseSmoke),
        "--runtime-idle-smoke" => Some(CliCommand::RuntimeIdleSmoke),
        "--runtime-idle-cpu-smoke" => Some(CliCommand::RuntimeIdleCpuSmoke),
        "--frame-scheduler-smoke" => Some(CliCommand::FrameSchedulerSmoke),
        _ => None,
    }
}
