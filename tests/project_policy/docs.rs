use std::{fs, path::Path};

use super::{
    docs_markers::{
        REQUIRED_README_COMPLETION_GAP_MARKERS, REQUIRED_RELEASE_DOC_MARKERS,
        REQUIRED_VISUAL_CONTRACT_DOC_MARKERS,
    },
    support::relative_path,
};

#[test]
fn public_docs_keep_default_visual_contract_and_proof_commands() {
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
fn public_docs_keep_release_install_boundaries() {
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
