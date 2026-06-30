use std::{fs, path::Path};

use super::support::relative_path;

const REQUIRED_SITE_FILES: &[&str] = &[
    "site/index.html",
    "site/styles.css",
    "site/scripts.js",
    "site/check-links.mjs",
    "site/CNAME",
    "site/robots.txt",
    "site/sitemap.xml",
    "site/site.webmanifest",
    "site/og-image.png",
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
fn pages_site_keeps_gromaq_dev_seo_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let html_path = root.join("site/index.html");
    let html = fs::read_to_string(&html_path).unwrap();
    let cname = fs::read_to_string(root.join("site/CNAME")).unwrap();
    let robots = fs::read_to_string(root.join("site/robots.txt")).unwrap();
    let sitemap = fs::read_to_string(root.join("site/sitemap.xml")).unwrap();
    let manifest = fs::read_to_string(root.join("site/site.webmanifest")).unwrap();

    assert_eq!(cname.trim(), "gromaq.dev");

    for marker in [
        "<link rel=\"canonical\" href=\"https://gromaq.dev/\">",
        "<meta name=\"robots\" content=\"index, follow, max-image-preview:large\">",
        "<meta property=\"og:url\" content=\"https://gromaq.dev/\">",
        "<meta property=\"og:site_name\" content=\"Gromaq\">",
        "<meta property=\"og:image\" content=\"https://gromaq.dev/og-image.png\">",
        "<meta property=\"og:image:width\" content=\"1200\">",
        "<meta property=\"og:image:height\" content=\"630\">",
        "<meta property=\"og:image:alt\" content=\"Gromaq native Rust GPU terminal preview\">",
        "<meta name=\"twitter:card\" content=\"summary_large_image\">",
        "<meta name=\"twitter:image\" content=\"https://gromaq.dev/og-image.png\">",
        "<script type=\"application/ld+json\">",
        "\"url\": \"https://gromaq.dev/\"",
        "\"softwareVersion\": \"0.2.1\"",
    ] {
        assert!(
            html.contains(marker),
            "{} must include SEO marker `{marker}`",
            relative_path(root, &html_path)
        );
    }

    assert!(robots.contains("Sitemap: https://gromaq.dev/sitemap.xml"));
    assert!(sitemap.contains("<loc>https://gromaq.dev/</loc>"));
    assert!(manifest.contains("\"start_url\": \"https://gromaq.dev/\""));
    assert!(manifest.contains("\"name\": \"Gromaq\""));

    let (width, height) = png_dimensions(&root.join("site/og-image.png"));
    assert_eq!((width, height), (1200, 630));
}

#[test]
fn pages_site_keeps_real_terminal_media_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let html_path = root.join("site/index.html");
    let html = fs::read_to_string(&html_path).unwrap();

    for marker in [
        "class=\"terminal-screenshot\"",
        "src=\"assets/gromaq-welcome-preview.png\"",
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
            html.contains("class=\"terminal-screenshot\""),
            "{} must render the welcome preview as the intentional static screenshot",
            relative_path(root, &html_path)
        );
        for removed_marker in [
            "poster=\"",
            "fallback",
            "data-recording-status=\"blocked\"",
            "Recording capture blocked:",
            "macOS Screen Recording permission",
            "real native app recording is claimed",
        ] {
            assert!(
                !html.contains(removed_marker),
                "{} must omit recording fallback marker `{removed_marker}` when using the static screenshot",
                relative_path(root, &html_path)
            );
        }
    }
}

fn png_dimensions(path: &Path) -> (u32, u32) {
    let bytes = fs::read(path).unwrap();
    assert!(
        bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "{} must be a PNG image",
        relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), path)
    );
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    (width, height)
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
