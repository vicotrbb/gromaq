use std::{fs, path::Path};

use super::support::relative_path;

#[test]
fn welcome_preview_proof_keeps_default_visual_quality_floors() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("scripts/prove-welcome-preview.sh");
    let source = fs::read_to_string(&path).unwrap();

    for marker in [
        "require_min_metric \"high contrast text pixels\" 30000",
        "require_min_metric \"avatar color pixels\" 150000",
        "require_min_metric \"glyph quads\" 640",
        "require_exact_metric \"cursor quads\" 0",
        "require_log_marker \"frame size: 1468x820\"",
        "require_ppm_dimensions \"${ppm_path}\" 1468 820",
        "run_logged \"${log_path}\" cargo run -- --welcome-preview-snapshot",
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
    ] {
        assert!(
            source.contains(marker),
            "{} must keep `{marker}` so theme preview proof covers default and configured visual output",
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
        "cargo fmt --check",
        "git diff --check",
        "git diff --cached --check",
        "cargo clippy --all-targets --all-features -- -D warnings",
        "cargo test --all",
        "scripts/prove-theme-preview.sh",
        "node images/avatar/generate.mjs --check",
        "scripts/prove-welcome-preview.sh",
        "scripts/prove-readme-welcome-preview.sh",
        "scripts/prove-current-host-compatibility.sh",
        "cargo bench --bench parser_throughput -- --list",
    ] {
        assert!(
            source.contains(marker),
            "{} must keep `{marker}` so local push proof stays aligned with CI gates",
            relative_path(root, &path)
        );
    }
}
