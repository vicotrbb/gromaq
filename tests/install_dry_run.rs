//! Installer dry-run proof for supported platform plans.
//!
//! These tests keep the dry-run contract separate from artifact assembly so the
//! packaging tests stay scoped to tarball, `.deb`, and `.app` creation.

#![forbid(unsafe_code)]

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
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
    fn install_script_dry_run_reports_linux_release_asset_without_writes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let install_root = TempPath::new("gromaq-release-dry-run-root");
        let bin_dir = TempPath::new("gromaq-release-dry-run-bin");

        let output = Command::new("sh")
            .arg(root.join("scripts/install.sh"))
            .env("GROMAQ_DRY_RUN", "1")
            .env("GROMAQ_PLATFORM", "Linux")
            .env("GROMAQ_INSTALL_METHOD", "release")
            .env("GROMAQ_VERSION", "v0.1.0")
            .env("GROMAQ_RELEASE_TARGET", "linux-x86_64")
            .env("GROMAQ_BIN_DIR", bin_dir.path())
            .env("GROMAQ_INSTALL_ROOT", install_root.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "release dry-run install failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Dry run: would install gromaq"));
        assert!(stdout.contains("Dry run: would download release asset"));
        assert!(stdout.contains("gromaq-0.1.0-linux-x86_64.tar.gz"));
        assert!(stdout.contains("Dry run complete; no files written."));
        assert!(!install_root.path().exists());
        assert!(!bin_dir.path().exists());
    }

    #[test]
    fn install_script_installs_linux_release_tarball_from_local_base() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let release_dir = TempPath::new("gromaq-release-assets");
        let install_root = TempPath::new("gromaq-release-install-root");
        let bin_dir = TempPath::new("gromaq-release-bin");
        let stub = release_dir.path().join("gromaq-stub");
        fs::create_dir_all(release_dir.path()).unwrap();
        fs::write(&stub, "#!/bin/sh\nprintf 'stub gromaq\\n'\n").unwrap();
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();

        let package = Command::new("sh")
            .arg(root.join("scripts/package-linux-tarball.sh"))
            .env("GROMAQ_BINARY_PATH", &stub)
            .env("GROMAQ_DIST_DIR", release_dir.path())
            .env("GROMAQ_RELEASE_TARGET", "linux-x86_64")
            .output()
            .unwrap();
        assert!(
            package.status.success(),
            "release tarball packaging failed: {}",
            String::from_utf8_lossy(&package.stderr)
        );
        let checksums = Command::new("sh")
            .arg(root.join("scripts/generate-checksums.sh"))
            .env("GROMAQ_DIST_DIR", release_dir.path())
            .output()
            .unwrap();
        assert!(
            checksums.status.success(),
            "release checksum generation failed: {}",
            String::from_utf8_lossy(&checksums.stderr)
        );
        fs::copy(
            release_dir.path().join("SHA256SUMS"),
            release_dir.path().join("SHA256SUMS-linux-x86_64"),
        )
        .unwrap();

        let output = Command::new("sh")
            .arg(root.join("scripts/install.sh"))
            .env("GROMAQ_PLATFORM", "Linux")
            .env("GROMAQ_INSTALL_METHOD", "release")
            .env("GROMAQ_VERSION", "v0.1.0")
            .env("GROMAQ_RELEASE_TARGET", "linux-x86_64")
            .env(
                "GROMAQ_RELEASE_BASE",
                format!("file://{}", release_dir.path().display()),
            )
            .env("GROMAQ_BIN_DIR", bin_dir.path())
            .env("GROMAQ_INSTALL_ROOT", install_root.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "release tarball install failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        assert!(bin_dir.path().join("gromaq").is_file());
        assert!(
            install_root
                .path()
                .join("share/applications/dev.gromaq.Gromaq.desktop")
                .is_file()
        );
        assert!(
            install_root
                .path()
                .join("share/metainfo/dev.gromaq.Gromaq.metainfo.xml")
                .is_file()
        );
        assert!(
            install_root
                .path()
                .join("share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png")
                .is_file()
        );
    }

    #[test]
    fn install_script_refreshes_linux_desktop_database_when_available() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let install_root = TempPath::new("gromaq-linux-desktop-refresh-root");
        let tools_dir = TempPath::new("gromaq-linux-desktop-refresh-tools");
        let refresh_log = TempPath::new("gromaq-linux-desktop-refresh-log");
        fs::create_dir_all(tools_dir.path()).unwrap();
        let refresh_tool = tools_dir.path().join("update-desktop-database");
        fs::write(
            &refresh_tool,
            "#!/bin/sh\nprintf '%s\\n' \"$1\" > \"$GROMAQ_DESKTOP_REFRESH_LOG\"\n",
        )
        .unwrap();
        fs::set_permissions(&refresh_tool, fs::Permissions::from_mode(0o755)).unwrap();
        let path = format!(
            "{}:{}",
            tools_dir.path().display(),
            std::env::var("PATH").unwrap_or_default()
        );

        let output = Command::new("sh")
            .arg(root.join("scripts/install.sh"))
            .env("GROMAQ_SKIP_CARGO_INSTALL", "1")
            .env("GROMAQ_PLATFORM", "Linux")
            .env("GROMAQ_ASSET_ROOT", root)
            .env("GROMAQ_INSTALL_ROOT", install_root.path())
            .env("GROMAQ_DESKTOP_REFRESH_LOG", refresh_log.path())
            .env("PATH", path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "desktop refresh install failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let applications_dir = install_root.path().join("share/applications");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(&format!(
                "Refreshed Linux desktop database under {}.",
                applications_dir.display()
            )),
            "installer should report the desktop database refresh:\n{stdout}"
        );
        assert_eq!(
            fs::read_to_string(refresh_log.path()).unwrap().trim(),
            applications_dir.to_string_lossy()
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
