use std::{fs, path::Path};

use super::support::relative_path;

const REQUIRED_CI_COMMANDS: &[&str] = &[
    "sh -n scripts/install.sh",
    "sh -n scripts/package-macos-app.sh",
    "sh -n scripts/package-linux-tarball.sh",
    "sh -n scripts/package-debian-deb.sh",
    "sh -n scripts/generate-checksums.sh",
    "sh -n scripts/notarize-macos-app.sh",
    "sh -n scripts/capture-macos-window-proof.sh",
    "bash -n packaging/arch/PKGBUILD",
    "cargo fmt --check",
    "git diff --check",
    "cargo clippy --all-targets --all-features -- -D warnings",
    "cargo test --all",
    "cargo run -- --theme-legibility-smoke",
    "cargo run -- --theme-preview-snapshot target/gromaq-theme-preview-ci.ppm",
    "cargo run -- --theme-preview-config target/gromaq-theme-preview-config-ci.toml target/gromaq-theme-preview-config-ci.ppm",
    "cursor_opacity = 0.5",
    "selection_opacity = 0.25",
    "cargo run -- --runtime-clipboard-paste-smoke",
    "cargo run -- --runtime-glyph-frame-smoke",
    "cargo run -- --runtime-glyph-frame-snapshot target/gromaq-runtime-glyph-frame-ci.ppm",
    "cargo run -- --runtime-scrollback-smoke",
    "cargo run -- --runtime-perf-smoke",
    "cargo run -- --runtime-perf-budget-smoke",
    "cargo run -- --runtime-perf-p95-smoke",
    "cargo run -- --runtime-large-output-smoke",
    "cargo run -- --runtime-bounded-state-smoke",
    "cargo run -- --runtime-memory-smoke",
    "cargo run -- --runtime-continuous-output-smoke",
    "cargo run -- --runtime-alternate-screen-smoke",
    "cargo run -- --runtime-reflow-smoke",
    "cargo run -- --runtime-config-reload-smoke",
    "cargo run -- --runtime-text-zoom-smoke",
    "cargo run -- --runtime-repaint-smoke",
    "cargo run -- --runtime-tool-workflow-smoke",
    "cargo run -- --runtime-focus-smoke",
    "cargo run -- --runtime-mouse-smoke",
    "cargo run -- --runtime-response-smoke",
    "cargo run -- --runtime-idle-smoke",
    "cargo run -- --runtime-idle-cpu-smoke",
    "cargo run -- --runtime-real-shell-smoke",
    "cargo run -- --runtime-real-shell-command-output-smoke",
    "cargo run -- --runtime-real-shell-perf-budget-smoke",
    "cargo run -- --runtime-real-shell-large-output-smoke",
    "cargo run -- --runtime-real-shell-reflow-smoke",
    "cargo bench --bench parser_throughput -- --list",
];

const REQUIRED_RELEASE_WORKFLOW_COMMANDS: &[&str] = &[
    "scripts/package-linux-tarball.sh",
    "scripts/package-debian-deb.sh",
    "scripts/package-macos-app.sh",
    "scripts/generate-checksums.sh",
    "GROMAQ_CHECKSUM_EXTRA_FILES=\"packaging/arch/PKGBUILD packaging/arch/.SRCINFO\" scripts/generate-checksums.sh",
    "actions/upload-artifact@v4",
    "target/dist/SHA256SUMS",
    "target/dist/*.tar.gz",
    "target/dist/*.deb",
    "packaging/arch/PKGBUILD",
    "packaging/arch/.SRCINFO",
    "target/dist/Gromaq-macos-app.zip",
];

const REQUIRED_TAG_RELEASE_UPLOAD_MARKERS: &[&str] = &[
    "permissions:",
    "contents: write",
    "startsWith(github.ref, 'refs/tags/')",
    "gh release create",
    "gh release upload",
    "packaging/arch/PKGBUILD",
    "packaging/arch/.SRCINFO",
    "SHA256SUMS-linux-x86_64",
    "SHA256SUMS-macos-app",
    "GH_TOKEN: ${{ github.token }}",
];

const REQUIRED_LINUX_PACKAGING_CI_MARKERS: &[&str] = &[
    "linux-packaging:",
    "runs-on: ubuntu-latest",
    "cargo test --test project_policy",
    "GROMAQ_SKIP_CARGO_INSTALL=1 GROMAQ_PLATFORM=Linux GROMAQ_ASSET_ROOT=\"$PWD\" GROMAQ_INSTALL_ROOT=target/install-proof sh scripts/install.sh",
    "scripts/package-linux-tarball.sh",
    "scripts/package-debian-deb.sh",
    "GROMAQ_CHECKSUM_EXTRA_FILES=\"packaging/arch/PKGBUILD packaging/arch/.SRCINFO\" scripts/generate-checksums.sh",
    "bash -n packaging/arch/PKGBUILD",
    "GROMAQ_INSTALL_METHOD=release GROMAQ_VERSION=v0.1.0",
    "GROMAQ_RELEASE_BASE=\"file://$PWD/target/dist\"",
    "GROMAQ_BIN_DIR=target/release-install-proof/bin",
    "test -x target/release-install-proof/bin/gromaq",
];

const REQUIRED_ARCH_PACKAGING_CI_MARKERS: &[&str] = &[
    "arch-packaging:",
    "container: archlinux:base-devel",
    "pacman -Syu --noconfirm git rust",
    "useradd -m builder",
    "chown -R builder:builder \"$PWD\"",
    "su builder -c \"cd '$PWD/packaging/arch' && makepkg --nobuild --noconfirm\"",
    "su builder -c \"cd '$PWD/packaging/arch' && makepkg --printsrcinfo\"",
    "test -s packaging/arch/.SRCINFO",
];

#[test]
fn ci_workflow_runs_required_root_checks() {
    let workflow_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap();

    for command in REQUIRED_CI_COMMANDS {
        assert!(
            workflow.contains(command),
            "{} must run `{command}`",
            relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), &workflow_path)
        );
    }
}

#[test]
fn release_workflow_builds_distribution_artifacts() {
    let workflow_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/release.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap();

    for command in REQUIRED_RELEASE_WORKFLOW_COMMANDS {
        assert!(
            workflow.contains(command),
            "{} must run or upload `{command}`",
            relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), &workflow_path)
        );
    }
}

#[test]
fn release_workflow_publishes_tag_assets_to_github_releases() {
    let workflow_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/release.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap();

    for marker in REQUIRED_TAG_RELEASE_UPLOAD_MARKERS {
        assert!(
            workflow.contains(marker),
            "{} must include tag release publication marker `{marker}`",
            relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), &workflow_path)
        );
    }
}

#[test]
fn ci_runs_linux_distribution_checks() {
    let workflow_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap();

    for marker in REQUIRED_LINUX_PACKAGING_CI_MARKERS {
        assert!(
            workflow.contains(marker),
            "{} must include Linux packaging marker `{marker}`",
            relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), &workflow_path)
        );
    }
}

#[test]
fn ci_runs_arch_packaging_checks() {
    let workflow_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap();

    for marker in REQUIRED_ARCH_PACKAGING_CI_MARKERS {
        assert!(
            workflow.contains(marker),
            "{} must include Arch packaging marker `{marker}`",
            relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), &workflow_path)
        );
    }
}
