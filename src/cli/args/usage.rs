/// User-facing usage text.
pub(in crate::cli) fn usage() -> String {
    concat!(
        "usage: gromaq [",
        "--help|-h|--version|-V|--gpu-info|--gpu-smoke|--gpu-upload-smoke|",
        "--gpu-glyph-atlas-smoke|",
        "--gpu-text-atlas-smoke|--gpu-textured-quad-smoke|",
        "--gpu-terminal-text-smoke|--gpu-terminal-text-perf-smoke|",
        "--gpu-terminal-text-snapshot <path>|--welcome-image-snapshot <path>|--clipboard-smoke|--config <path>|",
        "--config-check <path>|--config-template|--window-smoke|--window-perf-smoke|",
        "--window-screenshot-smoke|",
        "--window-glyph-frame-snapshot <path>|--osc52-clipboard-smoke|",
        "--runtime-clipboard-paste-smoke|--runtime-osc52-clipboard-smoke|",
        "--runtime-bracketed-paste-smoke|",
        "--runtime-selection-copy-smoke|",
        "--runtime-glyph-frame-smoke|",
        "--runtime-glyph-frame-snapshot <path>|--runtime-scrollback-smoke|",
        "--runtime-perf-smoke|--runtime-perf-budget-smoke|--runtime-perf-p95-smoke|",
        "--runtime-large-output-smoke|--runtime-bounded-state-smoke|",
        "--runtime-memory-smoke|--runtime-continuous-output-smoke|",
        "--runtime-real-shell-smoke|--runtime-real-shell-command-output-smoke|",
        "--runtime-real-shell-perf-budget-smoke|",
        "--runtime-real-shell-large-output-smoke|--runtime-real-shell-reflow-smoke|",
        "--runtime-alternate-screen-smoke|--runtime-reflow-smoke|",
        "--runtime-config-reload-smoke|--runtime-text-zoom-smoke|",
        "--runtime-repaint-smoke|--runtime-tool-workflow-smoke|",
        "--font-symbol-fallback-smoke|",
        "--theme-list|--theme-export <preset> <path>|",
        "--theme-legibility-smoke|--theme-preview-snapshot <path>|",
        "--theme-preview-config <config> <path>|",
        "--welcome-preview-snapshot <path>|",
        "--runtime-focus-smoke|--runtime-mouse-smoke|--runtime-response-smoke|",
        "--runtime-committed-text-smoke|--runtime-idle-smoke|--runtime-idle-cpu-smoke|",
        "--frame-scheduler-smoke]\n"
    )
    .to_owned()
}
