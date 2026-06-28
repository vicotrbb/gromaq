use std::{fs, path::Path};

const REQUIRED_GITHUB_RELEASE_ASSETS: &[&str] = &[
    "gromaq-${version_without_prefix}-linux-${release_arch}.tar.gz",
    "gromaq_${version_without_prefix}_${deb_arch}.deb",
    "PKGBUILD",
    ".SRCINFO",
    "gromaq.install",
    "Gromaq-macos-app.zip",
    "SHA256SUMS-linux-${release_arch}",
    "SHA256SUMS-macos-app",
];

#[test]
fn github_release_install_proof_checks_complete_published_asset_set() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proof_script =
        fs::read_to_string(root.join("scripts/prove-github-release-install.sh")).unwrap();

    assert!(proof_script.contains("gh release view"));
    assert!(proof_script.contains("--json assets"));
    assert!(proof_script.contains("verify_release_asset"));
    assert!(proof_script.contains("published release asset missing"));

    for asset in REQUIRED_GITHUB_RELEASE_ASSETS {
        assert!(
            proof_script.contains(asset),
            "GitHub release install proof must verify published asset `{asset}`"
        );
    }
}

#[test]
fn github_release_publication_proof_checks_tag_and_assets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proof_script =
        fs::read_to_string(root.join("scripts/prove-github-release-publication.sh")).unwrap();

    for marker in [
        "gh release view",
        "--json tagName,isDraft,isPrerelease,url,assets",
        "tagName",
        "isDraft",
        "verify_release_asset",
        "published release asset missing",
        "GitHub Release ${version} was not found",
        "gromaq-${version_without_prefix}-linux-${release_arch}.tar.gz",
        "gromaq_${version_without_prefix}_${deb_arch}.deb",
        "PKGBUILD",
        ".SRCINFO",
        "gromaq.install",
        "Gromaq-macos-app.zip",
        "SHA256SUMS-linux-${release_arch}",
        "SHA256SUMS-macos-app",
        "GitHub release publication proof: ok",
        "summary.txt",
    ] {
        assert!(
            proof_script.contains(marker),
            "GitHub release publication proof must include marker `{marker}`"
        );
    }
}
