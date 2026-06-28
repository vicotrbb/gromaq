use std::{fs, path::Path};

use super::support::relative_path;

const REQUIRED_TESTING_DOC_MARKERS: &[&str] = &[
    "scripts/prove-local-ci-parity.sh",
    "git diff --cached --check",
    "theme, avatar asset freshness, welcome, README screenshot",
];

const REQUIRED_LOCAL_VERIFICATION_DOC_MARKERS: &[(&str, &str)] = &[
    ("README.md", "scripts/prove-local-ci-parity.sh"),
    ("README.md", "staged and unstaged whitespace checks"),
    (
        "documentation/benchmarks.md",
        "scripts/prove-local-ci-parity.sh",
    ),
    ("documentation/benchmarks.md", "git diff --cached --check"),
    ("documentation/goal.md", "scripts/prove-local-ci-parity.sh"),
    ("documentation/goal.md", "git diff --cached --check"),
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
