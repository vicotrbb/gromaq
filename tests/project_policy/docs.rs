use std::{fs, path::Path};

use super::{
    docs_markers::{
        REQUIRED_README_COMPLETION_GAP_MARKERS, REQUIRED_README_LAUNCH_MARKERS,
        REQUIRED_RELEASE_DOC_MARKERS, REQUIRED_VISUAL_CONTRACT_DOC_MARKERS,
    },
    support::relative_path,
};

#[test]
fn readme_keeps_launch_release_shape() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("README.md");
    let source = fs::read_to_string(&path).unwrap();

    for marker in REQUIRED_README_LAUNCH_MARKERS {
        assert!(
            source.contains(marker),
            "{} must document `{marker}` for the public v0.2.1 launch page",
            relative_path(root, &path)
        );
    }
}

#[test]
fn detailed_docs_keep_default_visual_contract_and_proof_commands() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (relative, marker) in REQUIRED_VISUAL_CONTRACT_DOC_MARKERS {
        let path = root.join(relative);
        let source = fs::read_to_string(&path).unwrap();
        assert!(
            source.contains(marker),
            "{} must document `{marker}` for the default visual contract",
            relative_path(root, &path)
        );
    }
}

#[test]
fn detailed_docs_keep_release_install_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (relative, marker) in REQUIRED_RELEASE_DOC_MARKERS {
        let path = root.join(relative);
        let source = fs::read_to_string(&path).unwrap();
        assert!(
            source.contains(marker),
            "{} must document `{marker}` for public install and release boundaries",
            relative_path(root, &path)
        );
    }
}

#[test]
fn readme_keeps_known_completion_gaps_visible() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("README.md");
    let source = fs::read_to_string(&path).unwrap();

    for marker in REQUIRED_README_COMPLETION_GAP_MARKERS {
        assert!(
            source.contains(marker),
            "{} must keep `{marker}` visible in the not-yet-complete proof list",
            relative_path(root, &path)
        );
    }
}

#[test]
fn compatibility_matrix_rows_keep_three_columns() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("documentation/compatibility.md");
    let source = fs::read_to_string(&path).unwrap();

    for (line_number, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || trimmed == "| --- | --- | --- |" {
            continue;
        }

        assert_eq!(
            trimmed.matches('|').count(),
            4,
            "{}:{} must keep three markdown table columns",
            relative_path(root, &path),
            line_number + 1
        );
    }
}

#[test]
fn public_docs_avoid_drift_prone_current_head_remote_proof_claims() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for relative in [
        "README.md",
        "documentation/compatibility.md",
        "documentation/release.md",
    ] {
        let path = root.join(relative);
        let source = fs::read_to_string(&path).unwrap();
        assert!(
            !source.contains("current head"),
            "{} must cite exact commits or run ids instead of drift-prone `current head` remote proof claims",
            relative_path(root, &path)
        );
    }
}

#[test]
fn native_tmux_docs_track_ui_smoke_and_manual_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (relative, marker) in [
        (
            "documentation/compatibility.md",
            "destructive shortcut checked: true",
        ),
        (
            "documentation/compatibility.md",
            "unavailable shortcut blocked: true",
        ),
        (
            "documentation/compatibility.md",
            "mouse focus checked: true",
        ),
        (
            "documentation/compatibility.md",
            "mouse action selection checked: true",
        ),
        (
            "documentation/compatibility.md",
            "mouse workspace selection checked: true",
        ),
        (
            "documentation/compatibility.md",
            "startup manager after shell prompt checked: true",
        ),
        (
            "documentation/compatibility.md",
            "mouse clicks select session/window/pane/action/workspace rows",
        ),
        (
            "documentation/compatibility.md",
            "refresh shortcut requested: true",
        ),
        (
            "documentation/compatibility.md",
            "window cycle shortcuts checked: true",
        ),
        (
            "documentation/compatibility.md",
            "zoom shortcut checked: true",
        ),
        (
            "documentation/compatibility.md",
            "rename window action dispatched: true",
        ),
        (
            "documentation/compatibility.md",
            "split down shortcut checked: true",
        ),
        (
            "documentation/compatibility.md",
            "rename session action dispatched: true",
        ),
        (
            "documentation/compatibility.md",
            "kill pane confirmation dispatched: true",
        ),
        (
            "documentation/compatibility.md",
            "kill window confirmation dispatched: true",
        ),
        (
            "documentation/compatibility.md",
            "workspace feedback checked: true",
        ),
        (
            "documentation/compatibility.md",
            "cancellation feedback checked: true",
        ),
        (
            "documentation/architecture.md",
            "mouse-aware manager row hit testing",
        ),
        ("TESTING.md", "Press `r` and verify the manager refreshes"),
        (
            "TESTING.md",
            "Use `q` or another destructive shortcut and verify inline confirmation",
        ),
        ("TESTING.md", "startup shell output fills the first window"),
        (
            "TESTING.md",
            "Click session/window/pane/action/workspace rows",
        ),
        (
            "README.md",
            "`r` refreshes the tmux snapshot without shell input",
        ),
        ("README.md", "mouse clicks select visible manager rows"),
    ] {
        let path = root.join(relative);
        let source = fs::read_to_string(&path).unwrap();
        assert!(
            source.contains(marker),
            "{} must document `{marker}` for native tmux UI proof boundaries",
            relative_path(root, &path)
        );
    }
}

#[test]
fn readme_stays_concise_for_public_onboarding() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("README.md");
    let source = fs::read_to_string(&path).unwrap();
    let line_count = source.lines().count();

    assert!(
        (180..=260).contains(&line_count),
        "{} should stay roughly launch-page sized, saw {line_count} lines",
        relative_path(root, &path)
    );
}
