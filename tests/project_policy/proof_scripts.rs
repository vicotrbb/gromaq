use std::{fs, path::Path};

use super::support::relative_path;

#[test]
fn welcome_preview_proof_keeps_default_visual_quality_floors() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("scripts/prove-welcome-preview.sh");
    let source = fs::read_to_string(&path).unwrap();

    for marker in [
        "require_min_metric \"high contrast text pixels\" 22000",
        "require_min_metric \"avatar color pixels\" 20000",
        "require_min_metric \"glyph quads\" 640",
        "require_exact_metric \"cursor quads\" 0",
        "require_avatar_rows 17",
        "write_static_metric \"avatar rows\" \"${avatar_rows}\"",
        "require_log_marker \"frame size: 1468x820\"",
        "require_ppm_dimensions \"${ppm_path}\" 1468 820",
        "run_logged \"${log_path}\" cargo run -- --welcome-preview-snapshot",
        "summary.txt",
        "metrics.txt",
        "Metric: ${line}",
        "Welcome preview proof: ok",
    ] {
        assert!(
            source.contains(marker),
            "{} must keep `{marker}` so the default welcome preview proof fails closed",
            relative_path(root, &path)
        );
    }
}

#[test]
fn theme_preview_proof_keeps_configured_visual_quality_path() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("scripts/prove-theme-preview.sh");
    let source = fs::read_to_string(&path).unwrap();

    for marker in [
        "preset = \"gromaq-graphite\"",
        "background_opacity = 0.75",
        "cursor_opacity = 0.5",
        "selection_opacity = 0.25",
        "check_theme_preview_log \"${default_log}\" \"gromaq-ghostty\" 100 100 100",
        "check_theme_preview_log \"${config_log}\" \"gromaq-graphite\" 75 50 25",
        "run_logged \"${default_log}\" cargo run -- --theme-preview-snapshot",
        "run_logged \"${config_log}\" cargo run -- --theme-preview-config",
        "require_min_metric \"${log_path}\" \"high contrast text pixels\" 9000",
        "require_ppm_dimensions \"${default_ppm}\" 1036 292",
        "require_ppm_dimensions \"${config_ppm}\" 1036 292",
        "require_min_metric \"${log_path}\" \"prepared quads\" 100",
        "summary.txt",
        "metrics.txt",
        "Metric: ${line}",
        "Theme preview proof: ok",
    ] {
        assert!(
            source.contains(marker),
            "{} must keep `{marker}` so theme preview proof covers default and configured visual output",
            relative_path(root, &path)
        );
    }
}

#[test]
fn readme_welcome_preview_proof_writes_artifact_summary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("scripts/prove-readme-welcome-preview.sh");
    let source = fs::read_to_string(&path).unwrap();

    for marker in [
        "scripts/prove-welcome-preview.sh",
        "README welcome preview pixels: ok",
        "max_changed_pixels = 150_000",
        "max_mean_abs_delta = 8.0",
        "changed_pixels",
        "mean_abs_delta",
        "pixel-delta.txt",
        "Pixel delta: ${line}",
        "summary.txt",
        "README welcome preview proof: ok",
        "Committed PNG: ${readme_png}",
        "Generated PPM: ${ppm_path}",
    ] {
        assert!(
            source.contains(marker),
            "{} must keep `{marker}` so README screenshot freshness proof has stable artifact handles",
            relative_path(root, &path)
        );
    }
}

#[test]
fn local_ci_parity_proof_runs_clippy_before_completion() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("scripts/prove-local-ci-parity.sh");
    let source = fs::read_to_string(&path).unwrap();

    for marker in [
        "for script in scripts/*.sh",
        "sh -n \"${script}\"",
        "bash -n packaging/arch/PKGBUILD",
        "sh -n packaging/arch/gromaq.install",
        "cargo fmt --check",
        "git diff --check",
        "git diff --cached --check",
        "cargo clippy --all-targets --all-features -- -D warnings",
        "cargo test --all",
        "cargo run -- --font-symbol-fallback-smoke",
        "cargo run -- --runtime-bracketed-paste-smoke",
        "cargo run -- --runtime-selection-copy-smoke",
        "cargo run -- --runtime-committed-text-smoke",
        "cargo run -- --theme-legibility-smoke",
        "scripts/prove-theme-preview.sh",
        "node images/avatar/generate.mjs --check",
        "scripts/prove-welcome-preview.sh",
        "scripts/prove-readme-welcome-preview.sh",
        "cargo run -- --welcome-image-snapshot",
        "target/local-ci-parity-proof",
        "gromaq-welcome-image.ppm",
        "cargo run -- --gpu-terminal-text-smoke",
        "cargo run -- --frame-scheduler-smoke",
        "scripts/prove-current-host-compatibility.sh",
        "cargo bench --bench parser_throughput -- --list",
        "summary.txt",
        "Local CI parity proof: ok",
    ] {
        assert!(
            source.contains(marker),
            "{} must keep `{marker}` so local push proof stays aligned with CI gates",
            relative_path(root, &path)
        );
    }
}
