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
    "scripts/prove-macos-app-identity.sh",
    "scripts/prove-arch-package.sh",
    "scripts/prove-debian-package.sh",
    "scripts/prove-linux-release-install.sh",
    "scripts/prove-github-release-install.sh",
    "scripts/prove-linux-desktop-discovery.sh",
    "scripts/prove-current-host-compatibility.sh",
    "scripts/prove-144hz-window-perf.sh",
    "scripts/prove-theme-preview.sh",
    "scripts/prove-welcome-preview.sh",
    "scripts/prove-readme-welcome-preview.sh",
    "packaging/arch/PKGBUILD",
    "packaging/arch/.SRCINFO",
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
    let macos_identity_script =
        fs::read_to_string(root.join("scripts/prove-macos-app-identity.sh")).unwrap();
    let arch_proof_script = fs::read_to_string(root.join("scripts/prove-arch-package.sh")).unwrap();
    let debian_proof_script =
        fs::read_to_string(root.join("scripts/prove-debian-package.sh")).unwrap();
    let linux_release_proof_script =
        fs::read_to_string(root.join("scripts/prove-linux-release-install.sh")).unwrap();
    let github_release_proof_script =
        fs::read_to_string(root.join("scripts/prove-github-release-install.sh")).unwrap();
    let linux_desktop_discovery_script =
        fs::read_to_string(root.join("scripts/prove-linux-desktop-discovery.sh")).unwrap();
    let compatibility_proof_script =
        fs::read_to_string(root.join("scripts/prove-current-host-compatibility.sh")).unwrap();
    let pty_tools = fs::read_to_string(root.join("tests/pty/tools.rs")).unwrap();
    let window_perf_proof_script =
        fs::read_to_string(root.join("scripts/prove-144hz-window-perf.sh")).unwrap();
    let window_startup = fs::read_to_string(root.join("src/app/handler/resume.rs")).unwrap();
    let arch_pkgbuild = fs::read_to_string(root.join("packaging/arch/PKGBUILD")).unwrap();
    let arch_srcinfo = fs::read_to_string(root.join("packaging/arch/.SRCINFO")).unwrap();
    let arch_install = fs::read_to_string(root.join("packaging/arch/gromaq.install")).unwrap();
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
    assert!(macos_script.contains("LSApplicationCategoryType"));
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
    assert!(debian_script.contains("./postinst ./postrm"));
    assert!(debian_script.contains("update-desktop-database /usr/share/applications"));
    assert!(debian_script.contains("gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor"));
    assert!(debian_script.contains("data.tar.gz"));
    assert!(debian_script.contains(".deb"));
    assert!(arch_pkgbuild.contains("pkgname=gromaq-git"));
    assert!(arch_pkgbuild.contains("cargo build --release --locked"));
    assert!(arch_pkgbuild.contains("packaging/linux/dev.gromaq.Gromaq.desktop"));
    assert!(arch_pkgbuild.contains("dev.gromaq.Gromaq.metainfo.xml"));
    assert!(arch_pkgbuild.contains("logo-icon-256.png"));
    assert!(arch_pkgbuild.contains("install=gromaq.install"));
    assert!(arch_srcinfo.contains("pkgbase = gromaq-git"));
    assert!(arch_srcinfo.contains("pkgname = gromaq-git"));
    assert!(arch_srcinfo.contains("source = git+https://github.com/vicotrbb/gromaq.git"));
    assert!(arch_srcinfo.contains("install = gromaq.install"));
    assert!(arch_install.contains("post_install()"));
    assert!(arch_install.contains("update-desktop-database /usr/share/applications"));
    assert!(arch_install.contains("gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor"));
    assert!(checksum_script.contains("SHA256SUMS"));
    assert!(checksum_script.contains("GROMAQ_CHECKSUM_EXTRA_FILES"));
    assert!(checksum_script.contains(".tar.gz"));
    assert!(checksum_script.contains(".deb"));
    assert!(checksum_script.contains(".zip"));
    assert!(screenshot_script.contains("--window-screenshot-smoke"));
    assert!(screenshot_script.contains("cargo build"));
    assert!(screenshot_script.contains("${root}/target/debug/gromaq"));
    assert!(!screenshot_script.contains("--window-perf-smoke"));
    assert!(screenshot_script.contains("screencapture -x"));
    assert!(screenshot_script.contains("CGWindowListCopyWindowInfo"));
    assert!(screenshot_script.contains("CGWindowBounds"));
    assert!(screenshot_script.contains("kCGWindowSharingState"));
    assert!(screenshot_script.contains("CGPreflightScreenCaptureAccess"));
    assert!(screenshot_script.contains("macOS screen capture access preflight"));
    assert!(screenshot_script.contains("macOS window sharing state"));
    assert!(screenshot_script.contains("macOS window content is not shareable"));
    assert!(screenshot_script.contains("Gromaq"));
    assert!(screenshot_script.contains("screencapture -x -l"));
    assert!(screenshot_script.contains("screencapture -x -R"));
    assert!(screenshot_script.contains("window_capture_stderr"));
    assert!(screenshot_script.contains("region_capture_stderr"));
    assert!(screenshot_script.contains("NSBitmapImageRep"));
    assert!(screenshot_script.contains("GROMAQ_SCREENSHOT_DELAY_SECONDS:-0.05"));
    assert!(screenshot_script.contains("GROMAQ_SCREENSHOT_MIN_BACKGROUND_PIXELS"));
    assert!(screenshot_script.contains("GROMAQ_SCREENSHOT_MIN_FOREGROUND_PIXELS"));
    assert!(screenshot_script.contains("foreground sampled pixels"));
    assert!(screenshot_script.contains("rm -f \"${output}\""));
    assert!(macos_identity_script.contains("scripts/package-macos-app.sh"));
    assert!(macos_identity_script.contains("--window-screenshot-smoke"));
    assert!(macos_identity_script.contains("dev.gromaq.Gromaq"));
    assert!(macos_identity_script.contains("System Events"));
    assert!(macos_identity_script.contains("lsappinfo"));
    assert!(macos_identity_script.contains("LSDisplayName"));
    assert!(macos_identity_script.contains("Contents/MacOS/gromaq"));
    assert!(macos_identity_script.contains("macOS app identity proof: ok"));
    assert!(arch_proof_script.contains("archlinux:base-devel"));
    assert!(arch_proof_script.contains("packaging/arch/PKGBUILD"));
    assert!(arch_proof_script.contains("packaging/arch/gromaq.install"));
    assert!(arch_proof_script.contains("makepkg --noconfirm"));
    assert!(arch_proof_script.contains("pacman -U --noconfirm"));
    assert!(arch_proof_script.contains("/usr/bin/gromaq --version"));
    assert!(arch_proof_script.contains("pacman -Ql gromaq-git"));
    assert!(arch_proof_script.contains("dev.gromaq.Gromaq.desktop"));
    assert!(debian_proof_script.contains("scripts/package-debian-deb.sh"));
    assert!(debian_proof_script.contains("dpkg -i"));
    assert!(debian_proof_script.contains("\"/usr/bin/${package}\" --version"));
    assert!(debian_proof_script.contains("dpkg -L \"${package}\""));
    assert!(linux_release_proof_script.contains("scripts/package-linux-tarball.sh"));
    assert!(linux_release_proof_script.contains("scripts/generate-checksums.sh"));
    assert!(linux_release_proof_script.contains("GROMAQ_INSTALL_METHOD=release"));
    assert!(linux_release_proof_script.contains("GROMAQ_RELEASE_BASE=\"file://"));
    assert!(github_release_proof_script.contains("Linux"));
    assert!(github_release_proof_script.contains("GROMAQ_INSTALL_METHOD=release"));
    assert!(github_release_proof_script.contains("GROMAQ_VERSION"));
    assert!(
        github_release_proof_script
            .contains("https://github.com/vicotrbb/gromaq/releases/download")
    );
    assert!(github_release_proof_script.contains("GROMAQ_RELEASE_PROOF_ROOT"));
    assert!(github_release_proof_script.contains("GROMAQ_BIN_DIR=\"${proof_root}/bin\""));
    assert!(github_release_proof_script.contains("GROMAQ_INSTALL_ROOT=\"${proof_root}\""));
    assert!(github_release_proof_script.contains("GROMAQ_VERIFY_CHECKSUMS=1"));
    assert!(github_release_proof_script.contains("share/applications/dev.gromaq.Gromaq.desktop"));
    assert!(github_release_proof_script.contains("GitHub release install proof: ok"));
    assert!(
        linux_desktop_discovery_script.contains("Linux desktop discovery proof must run on Linux")
    );
    assert!(linux_desktop_discovery_script.contains("desktop-file-validate"));
    assert!(linux_desktop_discovery_script.contains("appstreamcli validate"));
    assert!(linux_desktop_discovery_script.contains("update-desktop-database"));
    assert!(linux_desktop_discovery_script.contains("gtk-update-icon-cache"));
    assert!(linux_desktop_discovery_script.contains("GROMAQ_SKIP_CARGO_INSTALL=1"));
    assert!(linux_desktop_discovery_script.contains("GROMAQ_INSTALL_ROOT"));
    assert!(linux_desktop_discovery_script.contains("dev.gromaq.Gromaq.desktop"));
    assert!(linux_desktop_discovery_script.contains("dev.gromaq.Gromaq.metainfo.xml"));
    assert!(linux_desktop_discovery_script.contains("dev.gromaq.Gromaq.png"));
    assert!(linux_desktop_discovery_script.contains("summary.txt"));
    assert!(linux_desktop_discovery_script.contains("Linux desktop discovery proof: ok"));
    assert!(linux_desktop_discovery_script.contains("does not prove live menu UI rendering"));
    assert!(compatibility_proof_script.contains("target/compatibility-proof"));
    assert!(compatibility_proof_script.contains("cargo test --test pty -- --nocapture"));
    assert!(compatibility_proof_script.contains("cargo run -- --runtime-tool-workflow-smoke"));
    assert!(compatibility_proof_script.contains("command -v"));
    assert!(compatibility_proof_script.contains("GROMAQ_REQUIRED_COMPAT_TOOLS"));
    assert!(compatibility_proof_script.contains("required compatibility tool missing"));
    assert!(compatibility_proof_script.contains("summary.txt"));
    assert!(compatibility_proof_script.contains("Current-host compatibility proof: ok"));
    assert!(pty_tools.contains("\"--output=yaml\""));
    assert!(window_perf_proof_script.contains("target/144hz-window-perf-proof"));
    assert!(window_perf_proof_script.contains("cargo run -- --window-perf-smoke"));
    assert!(window_perf_proof_script.contains("monitor refresh mhz"));
    assert!(window_perf_proof_script.contains("144000"));
    assert!(window_perf_proof_script.contains("frame interval target limited by monitor: false"));
    assert!(window_perf_proof_script.contains("frame pacing accepted: true"));
    assert!(window_perf_proof_script.contains("dropped frames: 0"));
    assert!(window_perf_proof_script.contains("144Hz window perf proof: ok"));
    assert!(window_startup.contains("screen_capture_allowed"));
    assert!(window_startup.contains("set_content_protected(!allowed)"));
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
