use std::{fs, path::Path};

use super::support::relative_path;

const REQUIRED_TESTING_DOC_MARKERS: &[&str] = &[
    "scripts/prove-local-ci-parity.sh",
    "git diff --cached --check",
    "shell syntax checks",
    "Avatar asset freshness",
    "README screenshot freshness",
    "default-startup tmux UI proof host",
    "default startup marker: tmux Cmd/Ctrl+Shift+T",
    "LaunchServices smoke stdout",
    "window screenshot smoke: ok",
    "live-app-window-proof.txt",
    "GROMAQ_SCREENSHOT_MIN_TMUX_PIXELS",
    "current-host compatibility",
];

const REQUIRED_LOCAL_VERIFICATION_DOC_MARKERS: &[(&str, &str)] = &[
    ("README.md", "scripts/prove-local-ci-parity.sh"),
    ("README.md", "staged and"),
    ("README.md", "unstaged whitespace"),
    ("README.md", "Avatar asset freshness"),
    (
        "documentation/benchmarks.md",
        "scripts/prove-local-ci-parity.sh",
    ),
    ("documentation/benchmarks.md", "git diff --cached --check"),
    ("documentation/goal.md", "scripts/prove-local-ci-parity.sh"),
    ("documentation/goal.md", "git diff --cached --check"),
    ("documentation/goal.md", "live-app-window-proof.txt"),
    ("TESTING.md", "scripts/prove-local-ci-parity.sh"),
    ("TESTING.md", "git diff --cached --check"),
];

#[test]
fn testing_docs_keep_local_parity_proof_visible() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("TESTING.md");
    let source = fs::read_to_string(&path).unwrap();

    for marker in REQUIRED_TESTING_DOC_MARKERS {
        assert!(
            source.contains(marker),
            "{} must document `{marker}` for local CI parity verification",
            relative_path(root, &path)
        );
    }
}

#[test]
fn public_docs_keep_local_verification_entry_points() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for (relative, marker) in REQUIRED_LOCAL_VERIFICATION_DOC_MARKERS {
        let path = root.join(relative);
        let source = fs::read_to_string(&path).unwrap();
        assert!(
            source.contains(marker),
            "{} must document `{marker}` for local verification",
            relative_path(root, &path)
        );
    }
}
