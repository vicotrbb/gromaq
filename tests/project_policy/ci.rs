use std::{fs, path::Path};

use super::support::relative_path;

const REQUIRED_CI_COMMANDS: &[&str] = &[
    "sh -n scripts/install.sh",
    "sh -n scripts/package-macos-app.sh",
    "sh -n scripts/package-linux-tarball.sh",
    "sh -n scripts/capture-macos-window-proof.sh",
    "cargo fmt --check",
    "git diff --check",
    "cargo clippy --all-targets --all-features -- -D warnings",
    "cargo test --all",
    "cargo run -- --theme-legibility-smoke",
    "cargo run -- --theme-preview-snapshot target/gromaq-theme-preview-ci.ppm",
    "cargo run -- --theme-preview-config target/gromaq-theme-preview-config-ci.toml target/gromaq-theme-preview-config-ci.ppm",
    "cursor_opacity = 0.5",
    "selection_opacity = 0.25",
    "cargo run -- --runtime-clipboard-paste-smoke",
    "cargo run -- --runtime-glyph-frame-smoke",
    "cargo run -- --runtime-glyph-frame-snapshot target/gromaq-runtime-glyph-frame-ci.ppm",
    "cargo run -- --runtime-scrollback-smoke",
    "cargo run -- --runtime-perf-smoke",
    "cargo run -- --runtime-perf-budget-smoke",
    "cargo run -- --runtime-perf-p95-smoke",
    "cargo run -- --runtime-large-output-smoke",
    "cargo run -- --runtime-bounded-state-smoke",
    "cargo run -- --runtime-memory-smoke",
    "cargo run -- --runtime-continuous-output-smoke",
    "cargo run -- --runtime-alternate-screen-smoke",
    "cargo run -- --runtime-reflow-smoke",
    "cargo run -- --runtime-config-reload-smoke",
    "cargo run -- --runtime-text-zoom-smoke",
    "cargo run -- --runtime-repaint-smoke",
    "cargo run -- --runtime-tool-workflow-smoke",
    "cargo run -- --runtime-focus-smoke",
    "cargo run -- --runtime-mouse-smoke",
    "cargo run -- --runtime-response-smoke",
    "cargo run -- --runtime-idle-smoke",
    "cargo run -- --runtime-idle-cpu-smoke",
    "cargo run -- --runtime-real-shell-smoke",
    "cargo run -- --runtime-real-shell-command-output-smoke",
    "cargo run -- --runtime-real-shell-perf-budget-smoke",
    "cargo run -- --runtime-real-shell-large-output-smoke",
    "cargo run -- --runtime-real-shell-reflow-smoke",
    "cargo bench --bench parser_throughput -- --list",
];

const REQUIRED_RELEASE_WORKFLOW_COMMANDS: &[&str] = &[
    "scripts/package-linux-tarball.sh",
    "scripts/package-macos-app.sh",
    "actions/upload-artifact@v4",
    "target/dist/*.tar.gz",
    "target/dist/Gromaq-macos-app.zip",
];

#[test]
fn ci_workflow_runs_required_root_checks() {
    let workflow_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap();

    for command in REQUIRED_CI_COMMANDS {
        assert!(
            workflow.contains(command),
            "{} must run `{command}`",
            relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), &workflow_path)
        );
    }
}

#[test]
fn release_workflow_builds_distribution_artifacts() {
    let workflow_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/release.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap();

    for command in REQUIRED_RELEASE_WORKFLOW_COMMANDS {
        assert!(
            workflow.contains(command),
            "{} must run or upload `{command}`",
            relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), &workflow_path)
        );
    }
}
