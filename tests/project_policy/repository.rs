use std::{fs, path::Path};

use toml::Value;

use super::support::relative_path;

const REQUIRED_REPOSITORY_FILES: &[&str] = &[
    "README.md",
    "ARCHITECTURE.md",
    "CONTRIBUTING.md",
    "CODE_OF_CONDUCT.md",
    "BENCHMARKS.md",
    "COMPATIBILITY.md",
    "ROADMAP.md",
    "LICENSE",
    "SECURITY.md",
    "TESTING.md",
    "DEBUGGING.md",
    "GOOD_FIRST_ISSUES.md",
    "scripts/install.sh",
    "scripts/package-macos-app.sh",
    "scripts/package-linux-tarball.sh",
    "scripts/package-debian-deb.sh",
    "scripts/generate-checksums.sh",
    "scripts/notarize-macos-app.sh",
    "scripts/capture-macos-window-proof.sh",
    "packaging/linux/dev.gromaq.Gromaq.desktop",
    "packaging/linux/dev.gromaq.Gromaq.metainfo.xml",
    "images/screenshots/gromaq-welcome-preview.png",
    "documentation/benchmarks.md",
    "documentation/release.md",
    "tests/fixtures/README.md",
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
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
fn repository_keeps_single_documentation_tree() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    assert!(
        root.join("documentation").is_dir(),
        "repository documentation must live under documentation/"
    );
    assert!(
        !root.join("docs").exists(),
        "do not keep a parallel docs/ tree; use documentation/ instead"
    );
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

#[test]
fn distribution_assets_keep_desktop_identity() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let install_script = fs::read_to_string(root.join("scripts/install.sh")).unwrap();
    let macos_script = fs::read_to_string(root.join("scripts/package-macos-app.sh")).unwrap();
    let linux_script = fs::read_to_string(root.join("scripts/package-linux-tarball.sh")).unwrap();
    let debian_script = fs::read_to_string(root.join("scripts/package-debian-deb.sh")).unwrap();
    let checksum_script = fs::read_to_string(root.join("scripts/generate-checksums.sh")).unwrap();
    let screenshot_script =
        fs::read_to_string(root.join("scripts/capture-macos-window-proof.sh")).unwrap();
    let desktop =
        fs::read_to_string(root.join("packaging/linux/dev.gromaq.Gromaq.desktop")).unwrap();
    let metainfo =
        fs::read_to_string(root.join("packaging/linux/dev.gromaq.Gromaq.metainfo.xml")).unwrap();

    assert!(install_script.contains("dev.gromaq.Gromaq.desktop"));
    assert!(install_script.contains("GROMAQ_INSTALL_DESKTOP_ASSETS"));
    assert!(install_script.contains("GROMAQ_SKIP_CARGO_INSTALL"));
    assert!(install_script.contains("GROMAQ_INSTALL_ROOT"));
    assert!(install_script.contains("GROMAQ_PLATFORM"));
    assert!(install_script.contains("GROMAQ_DRY_RUN"));
    assert!(install_script.contains("GROMAQ_INSTALL_APP_BUNDLE"));
    assert!(install_script.contains("GROMAQ_MACOS_APP_DIR"));
    assert!(install_script.contains("GROMAQ_INSTALL_METHOD"));
    assert!(install_script.contains("GROMAQ_RELEASE_BASE"));
    assert!(install_script.contains("GROMAQ_BIN_DIR"));
    assert!(install_script.contains("GROMAQ_VERIFY_CHECKSUMS"));
    assert!(install_script.contains("GROMAQ_CHECKSUM_ASSET"));
    assert!(install_script.contains("prepare_macos_asset_root"));
    assert!(install_script.contains("scripts/package-macos-app.sh"));
    assert!(macos_script.contains("CFBundleIconFile"));
    assert!(macos_script.contains("AppIcon.icns"));
    assert!(macos_script.contains("Cargo.toml"));
    assert!(macos_script.contains("CFBundleShortVersionString"));
    assert!(macos_script.contains("GROMAQ_CODESIGN_IDENTITY"));
    assert!(macos_script.contains("codesign --force --deep"));
    assert!(macos_script.contains("--options runtime --timestamp"));
    assert!(linux_script.contains("dev.gromaq.Gromaq.desktop"));
    assert!(linux_script.contains("logo-icon-256.png"));
    assert!(linux_script.contains(".tar.gz"));
    assert!(debian_script.contains("dev.gromaq.Gromaq.desktop"));
    assert!(debian_script.contains("control.tar.gz"));
    assert!(debian_script.contains("data.tar.gz"));
    assert!(debian_script.contains(".deb"));
    assert!(checksum_script.contains("SHA256SUMS"));
    assert!(checksum_script.contains(".tar.gz"));
    assert!(checksum_script.contains(".deb"));
    assert!(checksum_script.contains(".zip"));
    assert!(screenshot_script.contains("--window-screenshot-smoke"));
    assert!(!screenshot_script.contains("--window-perf-smoke"));
    assert!(screenshot_script.contains("screencapture -x"));
    assert!(screenshot_script.contains("CGWindowListCopyWindowInfo"));
    assert!(screenshot_script.contains("CGWindowBounds"));
    assert!(screenshot_script.contains("Gromaq"));
    assert!(screenshot_script.contains("screencapture -x -l"));
    assert!(screenshot_script.contains("screencapture -x -R"));
    assert!(screenshot_script.contains("NSBitmapImageRep"));
    assert!(screenshot_script.contains("GROMAQ_SCREENSHOT_MIN_BACKGROUND_PIXELS"));
    assert!(screenshot_script.contains("rm -f \"${output}\""));
    assert!(desktop.contains("Icon=dev.gromaq.Gromaq"));
    assert!(desktop.contains("Categories=System;TerminalEmulator;"));
    assert!(metainfo.contains("<id>dev.gromaq.Gromaq</id>"));
    assert!(metainfo.contains("<launchable type=\"desktop-id\">"));
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
