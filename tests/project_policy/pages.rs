use std::{fs, path::Path};

use super::support::relative_path;

const REQUIRED_SITE_FILES: &[&str] = &[
    "site/index.html",
    "site/styles.css",
    "site/scripts.js",
    "site/check-links.mjs",
    "site/assets/logo-on-graphite.png",
    "site/assets/logo-transparent.png",
    "site/assets/logo-icon-512.png",
    "site/assets/gromaq-welcome-preview.png",
    ".github/workflows/pages.yml",
    "scripts/capture-pages-terminal-recording.sh",
];

#[test]
fn pages_site_keeps_required_deployable_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in REQUIRED_SITE_FILES {
        assert!(
            root.join(relative).is_file(),
            "{relative} must exist for the GitHub Pages launch site"
        );
    }
}

#[test]
fn pages_site_keeps_real_terminal_media_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let html_path = root.join("site/index.html");
    let html = fs::read_to_string(&html_path).unwrap();

    for marker in [
        "poster=\"assets/gromaq-welcome-preview.png\"",
        "Native Rust GPU terminal",
        "GROMAQ_INSTALL_METHOD=release GROMAQ_VERSION=v0.2.1",
        "public alpha/beta",
        "Developer ID notarization is not claimed",
    ] {
        assert!(
            html.contains(marker),
            "{} must include `{marker}` for the proof-first launch page",
            relative_path(root, &html_path)
        );
    }

    let recording_path = root.join("site/assets/gromaq-terminal-recording.webm");
    if recording_path.is_file() {
        assert!(
            html.contains("<video"),
            "{} must render video controls when real recording media exists",
            relative_path(root, &html_path)
        );
        assert!(
            html.contains("assets/gromaq-terminal-recording.webm"),
            "{} must link the captured native terminal recording when the media file exists",
            relative_path(root, &html_path)
        );
    } else {
        assert!(
            !html.contains("<video"),
            "{} must not render broken video controls when recording media is absent",
            relative_path(root, &html_path)
        );
        assert!(
            html.contains("class=\"terminal-poster\""),
            "{} must render the welcome preview as the fallback media surface",
            relative_path(root, &html_path)
        );
        for marker in [
            "data-recording-status=\"blocked\"",
            "Recording capture blocked:",
            "macOS Screen Recording permission",
        ] {
            assert!(
                html.contains(marker),
                "{} must document the exact recording blocker when real media is absent",
                relative_path(root, &html_path)
            );
        }
    }
}

#[test]
fn pages_site_links_stay_inside_deploy_artifact() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let html_path = root.join("site/index.html");
    let html = fs::read_to_string(&html_path).unwrap();

    assert!(
        !html.contains("../"),
        "{} must not link outside the deployed site artifact",
        relative_path(root, &html_path)
    );
    assert!(
        html.contains("https://github.com/vicotrbb/gromaq/releases/tag/v0.2.1"),
        "{} must link directly to the published v0.2.1 release",
        relative_path(root, &html_path)
    );
}

#[test]
fn pages_site_omits_standalone_proof_boundaries_section() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let html_path = root.join("site/index.html");
    let html = fs::read_to_string(&html_path).unwrap();

    for removed_marker in [
        "href=\"#proof\"",
        "id=\"proof\"",
        "Proof boundaries",
        "Public alpha/beta means honest boundaries.",
        "boundary-grid",
    ] {
        assert!(
            !html.contains(removed_marker),
            "{} must omit the standalone proof-boundaries section marker `{removed_marker}`",
            relative_path(root, &html_path)
        );
    }
}

#[test]
fn pages_workflow_publishes_site_directory_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow_path = root.join(".github/workflows/pages.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap();

    for marker in [
        "actions/configure-pages@v5",
        "actions/upload-pages-artifact@v3",
        "actions/deploy-pages@v4",
        "path: site",
        "pages: write",
        "id-token: write",
        "node site/check-links.mjs",
    ] {
        assert!(
            workflow.contains(marker),
            "{} must include `{marker}` for Pages deployment",
            relative_path(root, &workflow_path)
        );
    }
}
