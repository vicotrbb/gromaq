use std::{fs, path::Path};

use toml::Value;

use super::support::relative_path;

const REQUIRED_REPOSITORY_FILES: &[&str] = &[
    "README.md",
    "ARCHITECTURE.md",
    "CONTRIBUTING.md",
    "BENCHMARKS.md",
    "COMPATIBILITY.md",
    "ROADMAP.md",
    "LICENSE",
    "TESTING.md",
    "DEBUGGING.md",
    "GOOD_FIRST_ISSUES.md",
    "scripts/install.sh",
    "documentation/benchmarks.md",
    "tests/fixtures/README.md",
    ".github/workflows/ci.yml",
    ".github/labels.yml",
    ".github/ISSUE_TEMPLATE/bug_report.md",
    ".github/ISSUE_TEMPLATE/compatibility_gap.md",
    ".github/ISSUE_TEMPLATE/performance_proof.md",
];

const REQUIRED_ISSUE_LABELS: &[&str] = &[
    "bug",
    "compatibility",
    "performance",
    "needs-proof",
    "needs-triage",
    "good first issue",
    "documentation",
    "tests",
    "gpu",
    "blocked-live-proof",
];

#[test]
fn repository_keeps_required_release_readiness_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for required_file in REQUIRED_REPOSITORY_FILES {
        let path = root.join(required_file);
        assert!(
            path.is_file(),
            "{required_file} must exist for repository release readiness"
        );
    }
}

#[test]
fn repository_keeps_required_issue_labels() {
    let labels_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/labels.yml");
    let labels = fs::read_to_string(&labels_path).unwrap();

    for label in REQUIRED_ISSUE_LABELS {
        let marker = format!("- name: {label}");
        assert!(
            labels.lines().any(|line| line.trim() == marker),
            "{} must define issue label `{label}`",
            relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), &labels_path)
        );
    }
}

#[test]
fn cargo_manifest_keeps_public_open_source_metadata() {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    let manifest: Value = toml::from_str(&manifest).unwrap();
    let package = manifest
        .get("package")
        .and_then(Value::as_table)
        .expect("Cargo.toml must define [package]");

    assert_eq!(
        package.get("license").and_then(Value::as_str),
        Some("MIT"),
        "Cargo package metadata must publish the license"
    );
    assert_eq!(
        package.get("homepage").and_then(Value::as_str),
        Some("https://gromaq.dev"),
        "Cargo package metadata must keep the product homepage"
    );
    assert_eq!(
        package.get("repository").and_then(Value::as_str),
        Some("https://github.com/vicotrbb/gromaq"),
        "Cargo package metadata must point contributors at the public source repository"
    );
    assert_eq!(
        package.get("readme").and_then(Value::as_str),
        Some("README.md"),
        "Cargo package metadata must expose the README"
    );
    assert_string_array_contains(package, "keywords", "terminal");
    assert_string_array_contains(package, "keywords", "wgpu");
    assert_string_array_contains(package, "categories", "command-line-utilities");
}

fn assert_string_array_contains(package: &toml::map::Map<String, Value>, field: &str, item: &str) {
    let values = package
        .get(field)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("Cargo package metadata must define `{field}`"));
    assert!(
        values.iter().any(|value| value.as_str() == Some(item)),
        "Cargo package metadata `{field}` must contain `{item}`"
    );
}
