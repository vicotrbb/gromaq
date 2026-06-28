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
    "sh -n scripts/prove-macos-app-identity.sh",
    "sh -n scripts/prove-arch-package.sh",
    "sh -n scripts/prove-debian-package.sh",
    "sh -n scripts/prove-linux-release-install.sh",
    "sh -n scripts/prove-github-release-install.sh",
    "sh -n scripts/prove-linux-desktop-discovery.sh",
    "sh -n scripts/prove-current-host-compatibility.sh",
    "sh -n scripts/prove-144hz-window-perf.sh",
    "sh -n scripts/prove-welcome-preview.sh",
    "bash -n packaging/arch/PKGBUILD",
    "sh -n packaging/arch/gromaq.install",
    "cargo fmt --check",
    "git diff --check",
    "cargo clippy --all-targets --all-features -- -D warnings",
    "cargo test --all",
    "cargo run -- --theme-legibility-smoke",
    "cargo run -- --theme-preview-snapshot target/gromaq-theme-preview-ci.ppm",
    "gromaq-theme-preview-proof",
    "target/gromaq-theme-preview-ci.ppm",
    "target/gromaq-theme-preview-config-ci.ppm",
    "target/gromaq-theme-preview-config-ci.toml",
    "node images/avatar/generate.mjs --check",
    "scripts/prove-current-host-compatibility.sh",
    "gromaq-current-host-compatibility-proof",
    "target/compatibility-proof/*",
    "scripts/prove-welcome-preview.sh",
    "gromaq-welcome-preview-proof",
    "target/welcome-preview-proof/*",
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
    "GROMAQ_CHECKSUM_EXTRA_FILES=\"packaging/arch/PKGBUILD packaging/arch/.SRCINFO packaging/arch/gromaq.install\" scripts/generate-checksums.sh",
    "actions/upload-artifact@v4",
    "include-hidden-files: true",
    "target/dist/SHA256SUMS",
    "target/dist/*.tar.gz",
    "target/dist/*.deb",
    "packaging/arch/PKGBUILD",
    "packaging/arch/.SRCINFO",
    "packaging/arch/gromaq.install",
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
    "packaging/arch/gromaq.install",
    "SHA256SUMS-linux-x86_64",
    "SHA256SUMS-macos-app",
    "GH_TOKEN: ${{ github.token }}",
];

const REQUIRED_LINUX_PACKAGING_CI_MARKERS: &[&str] = &[
    "linux-packaging:",
    "runs-on: ubuntu-latest",
    "sudo apt-get install -y desktop-file-utils appstream gtk-update-icon-cache",
    "cargo test --test project_policy",
    "GROMAQ_SKIP_CARGO_INSTALL=1 GROMAQ_PLATFORM=Linux GROMAQ_ASSET_ROOT=\"$PWD\" GROMAQ_INSTALL_ROOT=target/install-proof sh scripts/install.sh",
    "scripts/prove-linux-desktop-discovery.sh",
    "scripts/package-linux-tarball.sh",
    "scripts/package-debian-deb.sh",
    "sudo dpkg -i target/dist/gromaq_*.deb",
    "test -x /usr/bin/gromaq",
    "/usr/bin/gromaq --version",
    "dpkg -L gromaq",
    "/usr/share/doc/gromaq/README.md",
    "/usr/share/applications/dev.gromaq.Gromaq.desktop",
    "/usr/share/metainfo/dev.gromaq.Gromaq.metainfo.xml",
    "/usr/share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png",
    "GROMAQ_CHECKSUM_EXTRA_FILES=\"packaging/arch/PKGBUILD packaging/arch/.SRCINFO packaging/arch/gromaq.install\" scripts/prove-linux-release-install.sh",
    "bash -n packaging/arch/PKGBUILD",
    "test -x target/release-install-proof/bin/gromaq",
];

const REQUIRED_LINUX_COMPATIBILITY_CI_MARKERS: &[&str] = &[
    "runs-on: ubuntu-latest",
    "sudo apt-get install -y bash zsh fish vim neovim tmux less procps htop btop openssh-client",
    "scripts/prove-current-host-compatibility.sh",
    "GROMAQ_REQUIRED_COMPAT_TOOLS: bash zsh fish vim nvim tmux less top htop btop ssh",
    "gromaq-linux-compatibility-proof",
    "target/compatibility-proof/*",
];

const REQUIRED_ARCH_PACKAGING_CI_MARKERS: &[&str] = &[
    "arch-packaging:",
    "container: archlinux:base-devel",
    "pacman -Syu --noconfirm git rust",
    "useradd -m builder",
    "chown -R builder:builder \"$PWD\"",
    "su builder -c \"cd '$PWD/packaging/arch' && makepkg --nobuild --noconfirm\"",
    "su builder -c \"cd '$PWD/packaging/arch' && makepkg --noconfirm\"",
    "pacman -U --noconfirm packaging/arch/gromaq-git-*.pkg.tar.*",
    "test -x /usr/bin/gromaq",
    "/usr/bin/gromaq --version",
    "pacman -Ql gromaq-git",
    "/usr/share/applications/dev.gromaq.Gromaq.desktop",
    "/usr/share/metainfo/dev.gromaq.Gromaq.metainfo.xml",
    "/usr/share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png",
    "su builder -c \"cd '$PWD/packaging/arch' && makepkg --printsrcinfo\"",
    "test -s packaging/arch/.SRCINFO",
    "test -s packaging/arch/gromaq.install",
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
fn linux_packaging_job_runs_release_install_proof_helper() {
    let workflow_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap();
    let linux_job = workflow
        .split("  arch-packaging:")
        .next()
        .unwrap()
        .split("  linux-packaging:")
        .nth(1)
        .unwrap();

    assert!(
        linux_job.contains("scripts/prove-linux-release-install.sh"),
        "{} linux-packaging job must run the release install proof helper",
        relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), &workflow_path)
    );
}

#[test]
fn ci_runs_linux_compatibility_checks() {
    let workflow_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap();
    let linux_compatibility_job = workflow
        .split("  arch-packaging:")
        .next()
        .unwrap()
        .split("  linux-compatibility:")
        .nth(1)
        .unwrap();

    for marker in REQUIRED_LINUX_COMPATIBILITY_CI_MARKERS {
        assert!(
            linux_compatibility_job.contains(marker),
            "{} linux-compatibility job must include marker `{marker}`",
            relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), &workflow_path)
        );
    }
}

#[test]
fn ci_uploads_compatibility_proof_artifacts_after_failures() {
    let workflow_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap();

    for artifact_name in [
        "gromaq-current-host-compatibility-proof",
        "gromaq-linux-compatibility-proof",
    ] {
        let artifact_block = workflow
            .split(artifact_name)
            .next()
            .unwrap()
            .rsplit("uses: actions/upload-artifact@v4")
            .next()
            .unwrap();

        assert!(
            artifact_block.contains("if: always()"),
            "{} must upload `{artifact_name}` even when the proof command fails",
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
