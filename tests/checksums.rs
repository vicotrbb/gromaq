//! Functional proof for release checksum manifest generation.

#![forbid(unsafe_code)]

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn checksum_script_can_include_additional_release_assets() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let dist = TempDist(std::env::temp_dir().join(unique_temp_name("gromaq-checksums")));
        fs::create_dir_all(dist.path()).unwrap();
        fs::write(
            dist.path().join("gromaq-0.1.0-linux-x86_64.tar.gz"),
            b"stub archive",
        )
        .unwrap();

        let output = Command::new("sh")
            .arg(root.join("scripts/generate-checksums.sh"))
            .env("GROMAQ_DIST_DIR", dist.path())
            .env(
                "GROMAQ_CHECKSUM_EXTRA_FILES",
                root.join("packaging/arch/PKGBUILD"),
            )
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "checksum generation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let manifest = fs::read_to_string(dist.path().join("SHA256SUMS")).unwrap();
        assert!(
            manifest.contains("gromaq-0.1.0-linux-x86_64.tar.gz"),
            "manifest missing release tarball:\n{manifest}"
        );
        assert!(
            manifest.contains("PKGBUILD"),
            "manifest missing extra PKGBUILD asset:\n{manifest}"
        );
    }

    struct TempDist(PathBuf);

    impl TempDist {
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDist {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn unique_temp_name(prefix: &str) -> String {
        let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        format!(
            "{}-{}-{}-{}",
            prefix,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            sequence
        )
    }
}
