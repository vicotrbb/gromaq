//! CLI argument vocabulary and usage text.

/// Parsed top-level command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CliCommand<'a> {
    /// GPU-backed smoke or diagnostics command.
    Gpu(&'a str),
    /// Host clipboard smoke command.
    ClipboardSmoke,
    /// Config-file native launch command.
    Config,
    /// Config-file validation command.
    ConfigCheck,
    /// Starter config template command.
    ConfigTemplate,
    /// OSC 52 clipboard smoke command.
    Osc52ClipboardSmoke,
    /// Runtime clipboard paste smoke command.
    RuntimeClipboardPasteSmoke,
    /// Runtime glyph-frame smoke command.
    RuntimeGlyphFrameSmoke,
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
        "--clipboard-smoke" => Some(CliCommand::ClipboardSmoke),
        "--config" => Some(CliCommand::Config),
        "--config-check" => Some(CliCommand::ConfigCheck),
        "--config-template" => Some(CliCommand::ConfigTemplate),
        "--osc52-clipboard-smoke" => Some(CliCommand::Osc52ClipboardSmoke),
        "--runtime-clipboard-paste-smoke" => Some(CliCommand::RuntimeClipboardPasteSmoke),
        "--runtime-glyph-frame-smoke" => Some(CliCommand::RuntimeGlyphFrameSmoke),
        "--runtime-scrollback-smoke" => Some(CliCommand::RuntimeScrollbackSmoke),
        "--runtime-perf-smoke" => Some(CliCommand::RuntimePerfSmoke),
        "--runtime-perf-budget-smoke" => Some(CliCommand::RuntimePerfBudgetSmoke),
        "--runtime-perf-p95-smoke" => Some(CliCommand::RuntimePerfP95Smoke),
        "--runtime-large-output-smoke" => Some(CliCommand::RuntimeLargeOutputSmoke),
        "--runtime-bounded-state-smoke" => Some(CliCommand::RuntimeBoundedStateSmoke),
        "--runtime-memory-smoke" => Some(CliCommand::RuntimeMemorySmoke),
        "--runtime-continuous-output-smoke" => Some(CliCommand::RuntimeContinuousOutputSmoke),
        "--runtime-real-shell-smoke" => Some(CliCommand::RuntimeRealShellSmoke),
        "--runtime-real-shell-large-output-smoke" => {
            Some(CliCommand::RuntimeRealShellLargeOutputSmoke)
        }
        "--runtime-real-shell-reflow-smoke" => Some(CliCommand::RuntimeRealShellReflowSmoke),
        "--runtime-alternate-screen-smoke" => Some(CliCommand::RuntimeAlternateScreenSmoke),
        "--runtime-reflow-smoke" => Some(CliCommand::RuntimeReflowSmoke),
        "--runtime-config-reload-smoke" => Some(CliCommand::RuntimeConfigReloadSmoke),
        "--runtime-focus-smoke" => Some(CliCommand::RuntimeFocusSmoke),
        "--runtime-mouse-smoke" => Some(CliCommand::RuntimeMouseSmoke),
        "--runtime-response-smoke" => Some(CliCommand::RuntimeResponseSmoke),
        "--runtime-idle-smoke" => Some(CliCommand::RuntimeIdleSmoke),
        "--runtime-idle-cpu-smoke" => Some(CliCommand::RuntimeIdleCpuSmoke),
        "--frame-scheduler-smoke" => Some(CliCommand::FrameSchedulerSmoke),
        _ => None,
    }
}

/// User-facing usage text.
pub(super) fn usage() -> String {
    "usage: gromaq [--gpu-info|--gpu-smoke|--gpu-upload-smoke|--gpu-glyph-atlas-smoke|--gpu-text-atlas-smoke|--gpu-textured-quad-smoke|--gpu-terminal-text-smoke|--gpu-terminal-text-perf-smoke|--clipboard-smoke|--config <path>|--config-check <path>|--config-template|--osc52-clipboard-smoke|--runtime-clipboard-paste-smoke|--runtime-glyph-frame-smoke|--runtime-scrollback-smoke|--runtime-perf-smoke|--runtime-perf-budget-smoke|--runtime-perf-p95-smoke|--runtime-large-output-smoke|--runtime-bounded-state-smoke|--runtime-memory-smoke|--runtime-continuous-output-smoke|--runtime-real-shell-smoke|--runtime-real-shell-large-output-smoke|--runtime-real-shell-reflow-smoke|--runtime-alternate-screen-smoke|--runtime-reflow-smoke|--runtime-config-reload-smoke|--runtime-focus-smoke|--runtime-mouse-smoke|--runtime-response-smoke|--runtime-idle-smoke|--runtime-idle-cpu-smoke|--frame-scheduler-smoke]\n".to_owned()
}
