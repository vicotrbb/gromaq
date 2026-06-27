//! Release-asset installer proof beyond dry-run planning.

#![forbid(unsafe_code)]

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn install_script_rejects_linux_release_tarball_checksum_mismatch() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let release_dir = TempPath::new("gromaq-release-checksum-assets");
        let install_root = TempPath::new("gromaq-release-checksum-root");
        let bin_dir = TempPath::new("gromaq-release-checksum-bin");
        create_release_tarball(root, release_dir.path());
        fs::write(
            release_dir.path().join("SHA256SUMS-linux-x86_64"),
            "0000000000000000000000000000000000000000000000000000000000000000  gromaq-0.1.0-linux-x86_64.tar.gz\n",
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
            !output.status.success(),
            "checksum mismatch must fail installation"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("checksum mismatch"),
            "stderr should explain checksum mismatch: {stderr}"
        );
        assert!(
            !bin_dir.path().join("gromaq").exists(),
            "binary must not be installed after checksum failure"
        );
    }

    fn create_release_tarball(root: &Path, release_dir: &Path) {
        let stub = release_dir.join("gromaq-stub");
        fs::create_dir_all(release_dir).unwrap();
        fs::write(&stub, "#!/bin/sh\nprintf 'stub gromaq\\n'\n").unwrap();
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();

        let output = Command::new("sh")
            .arg(root.join("scripts/package-linux-tarball.sh"))
            .env("GROMAQ_BINARY_PATH", &stub)
            .env("GROMAQ_DIST_DIR", release_dir)
            .env("GROMAQ_RELEASE_TARGET", "linux-x86_64")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "release tarball packaging failed: {}",
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
}
