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
    let compat = "documentation/compatibility.md";
    for (relative, marker) in [
        (compat, "destructive shortcut checked: true"),
        (compat, "unavailable shortcut blocked: true"),
        (compat, "no-server start hint checked: true"),
        (compat, "outside attach hint checked: true"),
        (compat, "outside attach target checked: true"),
        (compat, "mouse focus checked: true"),
        (compat, "mouse action selection checked: true"),
        (compat, "mouse workspace selection checked: true"),
        (
            "documentation/compatibility.md",
            "startup manager after shell prompt checked: true",
        ),
        (
            "documentation/compatibility.md",
            "startup manager small-grid cells: 69x17",
        ),
        (
            "documentation/compatibility.md",
            "status pane command checked: true",
        ),
        (
            "documentation/compatibility.md",
            "status feedback checked: true",
        ),
        (compat, "status guidance checked: true"),
        (
            "documentation/compatibility.md",
            "target pane detail checked: true",
        ),
        (
            "documentation/compatibility.md",
            "status missing/no server/detached/attached in the manager header",
        ),
        (
            "documentation/compatibility.md",
            "manager header status checked: true",
        ),
        (compat, "quoted concrete attach guidance"),
        (
            "documentation/compatibility.md",
            "current pane marker checked: true",
        ),
        (
            "documentation/compatibility.md",
            "current session/window/pane row markers remain visible after selection moves",
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
            "refresh focus preserved: true",
        ),
        (
            "documentation/compatibility.md",
            "help catalog checked: true",
        ),
        (
            "documentation/compatibility.md",
            "help catalog action coverage checked: true",
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
            "select pane shortcut checked: true",
        ),
        (
            "documentation/compatibility.md",
            "rename window action dispatched: true",
        ),
        (
            "documentation/compatibility.md",
            "safe action dispatched: true",
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
            "kill session confirmation dispatched: true",
        ),
        (
            "documentation/compatibility.md",
            "name entry action dispatched: true",
        ),
        (
            "documentation/compatibility.md",
            "attach session pty handoff checked: true",
        ),
        (
            "documentation/compatibility.md",
            "detach session shortcut checked: true",
        ),
        (
            "documentation/compatibility.md",
            "detach session failure feedback checked: true",
        ),
        (
            "documentation/compatibility.md",
            "tmux missing feedback checked: true",
        ),
        ("documentation/compatibility.md", "tmux command:"),
        ("documentation/compatibility.md", "confirmation required:"),
        (
            "documentation/compatibility.md",
            "start session pty handoff checked: true",
        ),
        (
            "documentation/compatibility.md",
            "workspace feedback checked: true",
        ),
        (
            "documentation/compatibility.md",
            "workspace command hints checked: true",
        ),
        (
            "documentation/compatibility.md",
            "workspace failure feedback checked: true",
        ),
        (
            "documentation/compatibility.md",
            "workspace invalid preflight checked: true",
        ),
        (
            "documentation/compatibility.md",
            "workspace duplicate prevented: true",
        ),
        (
            "documentation/compatibility.md",
            "`--runtime-tmux-smoke` and `--runtime-tmux-ui-smoke` fail closed",
        ),
        (
            "TESTING.md",
            "Both runtime tmux smokes are fail-closed proof commands",
        ),
        (
            "documentation/compatibility.md",
            "close the startup-open manager with Esc before proving Cmd/Ctrl+Shift `T` reopens it",
        ),
        ("documentation/compatibility.md", "tmux-binary-markers.txt"),
        (
            "documentation/compatibility.md",
            "fails if the selected executable still contains the old keyboard/mouse/paste startup copy",
        ),
        (
            "documentation/compatibility.md",
            "tmux-manager-not-hint.txt",
        ),
        (compat, "tmux-state-visible.txt"),
        (compat, "tmux-native-control-plane.txt"),
        (compat, "presented frame limit: 3"),
        ("TESTING.md", "presented frame limit: 3"),
        (compat, "terminal cells:"),
        ("TESTING.md", "terminal cells:"),
        ("README.md", "terminal cells:"),
        (
            "documentation/compatibility.md",
            "tmux-default-cargo-run-native-control-plane.txt",
        ),
        ("documentation/compatibility.md", "manual-checklist.txt"),
        (
            "documentation/compatibility.md",
            "native terminal control, not web UI",
        ),
        ("TESTING.md", "tmux-binary-markers.txt"),
        ("TESTING.md", "tmux-manager-not-hint.txt"),
        ("TESTING.md", "tmux-state-visible.txt"),
        ("TESTING.md", "tmux-native-control-plane.txt"),
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
        ("README.md", "scripts/prove-native-tmux-default-snapshot.sh"),
        ("ROADMAP.md", "Default native tmux snapshot artifact proof"),
        (
            "documentation/compatibility.md",
            "No-server snapshots may report `tmux status pane command rendered: false` only with `tmux manager panes: 0`",
        ),
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
