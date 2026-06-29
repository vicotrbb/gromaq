use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn install_script_rejects_macos_release_zip_missing_checksum_entry() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let release_dir = TempPath::new("gromaq-macos-missing-checksum-assets");
    let app_dir = TempPath::new("gromaq-macos-missing-checksum-apps");
    create_macos_release_zip(release_dir.path(), "0.2.1");
    fs::write(
        release_dir.path().join("SHA256SUMS-macos-app"),
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  unrelated-file\n",
    )
    .unwrap();

    let output = Command::new("sh")
        .arg(root.join("scripts/install.sh"))
        .env("GROMAQ_PLATFORM", "Darwin")
        .env("GROMAQ_INSTALL_METHOD", "release")
        .env("GROMAQ_VERSION", "v0.2.1")
        .env(
            "GROMAQ_RELEASE_BASE",
            format!("file://{}", release_dir.path().display()),
        )
        .env("GROMAQ_MACOS_APP_DIR", app_dir.path())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "missing macOS checksum entry must fail installation"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("checksum manifest did not contain"),
        "stderr should explain missing checksum entry: {stderr}"
    );
    assert!(!app_dir.path().join("Gromaq.app").exists());
}

#[test]
fn install_script_installs_macos_release_app_from_zip() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let release_dir = TempPath::new("gromaq-macos-release-assets");
    let app_dir = TempPath::new("gromaq-macos-release-apps");
    create_macos_release_zip(release_dir.path(), "0.2.1");
    create_checksum_manifest(root, release_dir.path());
    fs::copy(
        release_dir.path().join("SHA256SUMS"),
        release_dir.path().join("SHA256SUMS-macos-app"),
    )
    .unwrap();

    let output = Command::new("sh")
        .arg(root.join("scripts/install.sh"))
        .env("GROMAQ_PLATFORM", "Darwin")
        .env("GROMAQ_INSTALL_METHOD", "release")
        .env("GROMAQ_VERSION", "v0.2.1")
        .env(
            "GROMAQ_RELEASE_BASE",
            format!("file://{}", release_dir.path().display()),
        )
        .env("GROMAQ_MACOS_APP_DIR", app_dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "macOS release app install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Installed Gromaq.app."),
        "stdout should report the installed macOS app: {stdout}"
    );
    assert!(
        stdout.contains("Open it from:"),
        "stdout should explain how to open the macOS app: {stdout}"
    );
    assert!(
        !stdout.contains("Run it with: gromaq"),
        "macOS app release install must not print CLI binary launch guidance: {stdout}"
    );
    let installed_app = app_dir.path().join("Gromaq.app");
    assert!(installed_app.join("Contents/Info.plist").is_file());
    assert!(installed_app.join("Contents/MacOS/gromaq").is_file());
    let version = Command::new(installed_app.join("Contents/MacOS/gromaq"))
        .arg("--version")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&version.stdout).trim(),
        "gromaq 0.2.1"
    );
}

fn create_macos_release_zip(release_dir: &Path, version: &str) {
    let app = release_dir.join("Gromaq.app");
    let macos = app.join("Contents/MacOS");
    fs::create_dir_all(&macos).unwrap();
    fs::write(
        app.join("Contents/Info.plist"),
        "<?xml version=\"1.0\"?><plist version=\"1.0\"><dict></dict></plist>\n",
    )
    .unwrap();
    let executable = macos.join("gromaq");
    fs::write(
        &executable,
        format!("#!/bin/sh\nprintf 'gromaq {version}\\n'\n"),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();

    let output = Command::new("zip")
        .args(["-qr", "Gromaq-macos-app.zip", "Gromaq.app"])
        .current_dir(release_dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "macOS release zip creation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_checksum_manifest(root: &Path, release_dir: &Path) {
    let output = Command::new("sh")
        .arg(root.join("scripts/generate-checksums.sh"))
        .env("GROMAQ_DIST_DIR", release_dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "release checksum generation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

struct TempPath(PathBuf);

impl TempPath {
    fn new(prefix: &str) -> Self {
        Self(std::env::temp_dir().join(format!(
            "{}-{}-{}",
            prefix,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
