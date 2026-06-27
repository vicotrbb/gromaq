//! Installer dry-run proof for supported platform plans.
//!
//! These tests keep the dry-run contract separate from artifact assembly so the
//! packaging tests stay scoped to tarball, `.deb`, and `.app` creation.

#![forbid(unsafe_code)]

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn install_script_dry_run_reports_actions_without_writing_install_root() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let install_root = TempPath::new("gromaq-install-dry-run");

        let output = Command::new("sh")
            .arg(root.join("scripts/install.sh"))
            .env("GROMAQ_DRY_RUN", "1")
            .env("GROMAQ_PLATFORM", "Linux")
            .env("GROMAQ_INSTALL_ROOT", install_root.path())
            .env("GROMAQ_ASSET_ROOT", root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "dry-run install failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Dry run: would install gromaq"));
        assert!(stdout.contains("Dry run: would run cargo install"));
        assert!(stdout.contains("Dry run: would install Linux desktop assets under"));
        assert!(stdout.contains("Dry run complete; no files written."));
        assert!(
            !install_root.path().exists(),
            "dry-run install must not create install root"
        );
    }

    #[test]
    fn install_script_dry_run_reports_macos_app_bundle_without_writing_app_dir() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let app_dir = TempPath::new("gromaq-macos-dry-run");

        let output = Command::new("sh")
            .arg(root.join("scripts/install.sh"))
            .env("GROMAQ_DRY_RUN", "1")
            .env("GROMAQ_PLATFORM", "Darwin")
            .env("GROMAQ_INSTALL_APP_BUNDLE", "1")
            .env("GROMAQ_MACOS_APP_DIR", app_dir.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "macOS dry-run install failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Dry run: would install gromaq"));
        assert!(stdout.contains("Dry run: would run cargo install"));
        assert!(stdout.contains("Dry run: would install macOS app bundle to"));
        assert!(stdout.contains("Gromaq.app"));
        assert!(stdout.contains("Dry run complete; no files written."));
        assert!(
            !app_dir.path().exists(),
            "macOS dry-run install must not create app directory"
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
}
