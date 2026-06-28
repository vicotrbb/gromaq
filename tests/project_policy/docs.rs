use std::{fs, path::Path};

use super::support::relative_path;

const REQUIRED_VISUAL_CONTRACT_DOC_MARKERS: &[(&str, &str)] = &[
    ("README.md", "size_px = 32.0"),
    ("README.md", "line_height_px = 44.0"),
    ("README.md", "background_opacity = 1.0"),
    ("README.md", "cursor_opacity = 1.0"),
    ("README.md", "selection_opacity = 1.0"),
    ("README.md", "surface_padding_px = 14"),
    ("README.md", "cell_spacing_px = 0"),
    ("README.md", "preset = \"gromaq-ghostty\""),
    ("README.md", "cargo run -- --theme-list"),
    ("README.md", "cargo run -- --theme-export gromaq-ghostty"),
    ("README.md", "cargo run -- --runtime-text-zoom-smoke"),
    ("README.md", "cargo run -- --theme-legibility-smoke"),
    (
        "README.md",
        "cargo run -- --theme-preview-snapshot target/gromaq-theme-preview.ppm",
    ),
    ("README.md", "cargo run -- --theme-preview-config"),
    (
        "README.md",
        "cargo run -- --welcome-preview-snapshot target/gromaq-welcome-preview.ppm",
    ),
    ("README.md", "selection opacity, before adopting it"),
    ("documentation/theme.md", "32 px font size"),
    ("documentation/theme.md", "44 px line height"),
    ("documentation/theme.md", "18 px automatic cell width"),
    ("documentation/theme.md", "background_opacity"),
    ("documentation/theme.md", "cursor_opacity"),
    ("documentation/theme.md", "selection_opacity"),
    ("documentation/theme.md", "built-in default is `14`"),
    ("documentation/theme.md", "cell_spacing_px"),
    ("documentation/theme.md", "Control/Super `+`"),
    ("documentation/theme.md", "Control/Super `0`"),
    (
        "documentation/theme.md",
        "`cargo run -- --runtime-text-zoom-smoke`",
    ),
    (
        "documentation/theme.md",
        "`cargo run -- --theme-legibility-smoke`",
    ),
    (
        "documentation/theme.md",
        "`cargo run -- --theme-preview-snapshot",
    ),
    (
        "documentation/theme.md",
        "`cargo run -- --theme-preview-config",
    ),
    (
        "documentation/theme.md",
        "`cargo run -- --welcome-preview-snapshot",
    ),
    ("documentation/theme.md", "gromaq --theme-list"),
    ("documentation/theme.md", "gromaq --theme-export"),
    ("documentation/theme.md", "gromaq --theme-preview-config"),
    (
        "documentation/theme.md",
        "gromaq --welcome-preview-snapshot",
    ),
    ("documentation/compatibility.md", "32/18/44 px"),
    ("documentation/compatibility.md", "37/21/51 px"),
    ("documentation/compatibility.md", "gromaq-ghostty"),
    (
        "documentation/compatibility.md",
        "--welcome-preview-snapshot <path>",
    ),
    (
        "documentation/compatibility.md",
        "input-to-render p95 budget ns: 10000000",
    ),
    (
        "documentation/compatibility.md",
        "runtime perf p95 smoke: ok",
    ),
];

const REQUIRED_RELEASE_DOC_MARKERS: &[(&str, &str)] = &[
    ("README.md", "GROMAQ_INSTALL_APP_BUNDLE=1"),
    ("README.md", "GROMAQ_DRY_RUN=1"),
    ("README.md", "GROMAQ_INSTALL_METHOD=release"),
    ("README.md", "GROMAQ_BIN_DIR"),
    ("README.md", "GROMAQ_VERIFY_CHECKSUMS=0"),
    ("README.md", "GROMAQ_CHECKSUM_EXTRA_FILES"),
    ("README.md", "packaging/arch/PKGBUILD"),
    ("README.md", "packaging/arch/.SRCINFO"),
    ("README.md", "packaging/arch/gromaq.install"),
    ("README.md", "arch-packaging"),
    ("README.md", "makepkg --nobuild"),
    ("README.md", "makepkg --noconfirm"),
    ("README.md", "pacman -U"),
    ("README.md", "scripts/prove-arch-package.sh"),
    ("README.md", "scripts/prove-debian-package.sh"),
    ("README.md", "GROMAQ_MACOS_APP_DIR"),
    (
        "README.md",
        "installer asset placement plus Linux tarball and Debian package assembly",
    ),
    (
        "README.md",
        "Debian `postinst`/`postrm` desktop refresh hooks",
    ),
    (
        "README.md",
        "Debian package install, `gromaq --version`, and installed-payload checks",
    ),
    ("README.md", "remote GitHub Actions release workflow"),
    (
        "README.md",
        "live tag-triggered GitHub Release asset publication",
    ),
    (
        "README.md",
        "live Linux release-method install from GitHub Release assets",
    ),
    (
        "README.md",
        "Linux desktop database refresh when `update-desktop-database` is available",
    ),
    ("README.md", "signed/notarized macOS app distribution"),
    ("documentation/release.md", "GROMAQ_INSTALL_APP_BUNDLE=1"),
    ("documentation/release.md", "GROMAQ_DRY_RUN=1"),
    ("documentation/release.md", "GROMAQ_INSTALL_METHOD=release"),
    ("documentation/release.md", "GROMAQ_BIN_DIR"),
    ("documentation/release.md", "GROMAQ_VERIFY_CHECKSUMS=0"),
    ("documentation/release.md", "GROMAQ_CHECKSUM_EXTRA_FILES"),
    ("documentation/release.md", "packaging/arch/PKGBUILD"),
    ("documentation/release.md", "packaging/arch/.SRCINFO"),
    ("documentation/release.md", "packaging/arch/gromaq.install"),
    ("documentation/release.md", "arch-packaging"),
    ("documentation/release.md", "makepkg --nobuild"),
    ("documentation/release.md", "makepkg --noconfirm"),
    ("documentation/release.md", "pacman -U"),
    ("documentation/release.md", "scripts/prove-arch-package.sh"),
    (
        "documentation/release.md",
        "scripts/prove-debian-package.sh",
    ),
    ("documentation/release.md", "GROMAQ_MACOS_APP_DIR"),
    (
        "documentation/release.md",
        "optional macOS app-bundle install path",
    ),
    (
        "documentation/release.md",
        "Debian `postinst` and `postrm` maintainer scripts",
    ),
    (
        "documentation/release.md",
        "Debian package install, `gromaq --version`, and installed-payload checks",
    ),
    (
        "documentation/release.md",
        "tag-triggered GitHub Release publication path is configured locally",
    ),
    (
        "documentation/release.md",
        "Arch `PKGBUILD` plus `.SRCINFO` source-package metadata is configured for CI",
    ),
    ("documentation/release.md", "include-hidden-files: true"),
    (
        "documentation/compatibility.md",
        "tests/checksums.rs::checksum_script_can_include_additional_release_assets",
    ),
    (
        "documentation/compatibility.md",
        "can include `PKGBUILD`, `.SRCINFO`, and `gromaq.install`",
    ),
    (
        "documentation/compatibility.md",
        "Debian `postinst`/`postrm` desktop metadata refresh",
    ),
    (
        "documentation/compatibility.md",
        "Debian package install proof is configured",
    ),
    (
        "documentation/compatibility.md",
        "Arch `gromaq.install` desktop metadata refresh",
    ),
    (
        "documentation/compatibility.md",
        "Add live tag-triggered GitHub Release publication proof.",
    ),
    (
        "documentation/compatibility.md",
        "Add live Linux release-method install proof from GitHub Release assets.",
    ),
    (
        "documentation/compatibility.md",
        "Add live Linux desktop menu UI discovery proof.",
    ),
    (
        "documentation/compatibility.md",
        "Add live desktop OS paste-menu workflow proof.",
    ),
    (
        "documentation/compatibility.md",
        "Add Developer ID signed and notarized macOS app distribution proof.",
    ),
    ("documentation/release.md", "28301610408"),
    ("README.md", "28308158338"),
    ("documentation/release.md", "28308158338"),
    ("documentation/compatibility.md", "28308158338"),
    ("documentation/compatibility.md", "28301610408"),
];

const REQUIRED_README_COMPLETION_GAP_MARKERS: &[&str] = &[
    "live desktop OS paste-menu workflow",
    "hardware-backed 144 Hz frame pacing proof on a 144 Hz-capable display",
    "live desktop screenshot proof across supported platforms",
    "live tag-triggered GitHub Release asset publication",
    "live Linux release-method install from GitHub Release assets",
    "live Linux desktop menu UI discovery",
    "wider compatibility matrix coverage across shells, editors, multiplexers",
    "Developer ID signed/notarized macOS app distribution",
];

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
