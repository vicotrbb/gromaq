//! Functional packaging proof for the release artifact assembly scripts.
//!
//! These tests run the packaging scripts with a tiny stub binary so they prove
//! bundle/archive assembly without a release build. The Linux tarball script is
//! exercised on every Unix CI host; the macOS `.app` script is additionally
//! guarded to macOS because it needs `iconutil` and `sips`. Without these
//! tests the macOS bundle structure was only verified at release time.

#![forbid(unsafe_code)]

#[cfg(unix)]
mod unix {
    use std::fs::{self, Permissions};
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn linux_tarball_script_assembles_complete_archive_from_stub_binary() {
        let dist = run_packaging_script("scripts/package-linux-tarball.sh", &[]);
        let archive = single_tarball(dist.path());
        let stem = archive_stem(&archive);
        let listing = tar_listing(&archive);

        for required in [
            "/bin/gromaq",
            "/README.md",
            "/LICENSE",
            "/share/applications/dev.gromaq.Gromaq.desktop",
            "/share/metainfo/dev.gromaq.Gromaq.metainfo.xml",
            "/share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png",
        ] {
            assert!(
                listing
                    .iter()
                    .any(|entry| entry == &format!("{stem}{required}")),
                "tarball missing {stem}{required}; listing:\n{}",
                listing.join("\n")
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_app_script_builds_well_formed_bundle_from_stub_binary() {
        let dist = run_packaging_script("scripts/package-macos-app.sh", &[]);
        let contents = dist.path().join("Gromaq.app").join("Contents");

        let plist_path = contents.join("Info.plist");
        assert!(plist_path.is_file(), "Info.plist missing from bundle");
        let plist = fs::read_to_string(&plist_path).unwrap();
        assert!(
            plist.contains("<string>dev.gromaq.Gromaq</string>"),
            "Info.plist must declare the bundle identifier"
        );
        assert!(
            plist.contains("<string>AppIcon</string>"),
            "Info.plist must declare the AppIcon resource"
        );

        assert_eq!(
            fs::read(contents.join("PkgInfo")).unwrap(),
            b"APPL????".as_slice(),
            "PkgInfo must carry the APPL signature"
        );
        assert!(
            contents.join("MacOS/gromaq").is_file(),
            "bundle executable missing"
        );
        assert!(
            contents.join("Resources/AppIcon.icns").is_file(),
            "bundle icon missing"
        );

        let lint_ok = Command::new("plutil")
            .args(["-lint", &plist_path.to_string_lossy()])
            .status()
            .unwrap()
            .success();
        assert!(lint_ok, "Info.plist must pass plutil -lint");

        let icon_output = Command::new("file")
            .arg(contents.join("Resources/AppIcon.icns"))
            .output()
            .unwrap();
        let icon_kind = String::from_utf8_lossy(&icon_output.stdout);
        assert!(
            icon_kind.contains("icon"),
            "AppIcon.icns must be a macOS icon: {icon_kind}"
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

    fn run_packaging_script(script: &str, extra_env: &[(&str, &str)]) -> TempDist {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let dist = TempDist(std::env::temp_dir().join(format!(
            "gromaq-packaging-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        let stub = dist.path().join("gromaq-stub");
        fs::create_dir_all(dist.path()).unwrap();
        fs::write(&stub, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&stub, Permissions::from_mode(0o755)).unwrap();

        let output = Command::new("sh")
            .arg(root.join(script))
            .envs(extra_env.iter().copied())
            .env("GROMAQ_BINARY_PATH", &stub)
            .env("GROMAQ_DIST_DIR", dist.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} failed: {}",
            script,
            String::from_utf8_lossy(&output.stderr)
        );
        dist
    }

    fn single_tarball(dist: &Path) -> PathBuf {
        let mut tarballs: Vec<_> = fs::read_dir(dist)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "gz"))
            .collect();
        assert_eq!(
            tarballs.len(),
            1,
            "expected exactly one tarball: {tarballs:?}"
        );
        tarballs.pop().unwrap()
    }

    fn archive_stem(tarball: &Path) -> String {
        tarball
            .file_name()
            .unwrap()
            .to_string_lossy()
            .trim_end_matches(".tar.gz")
            .to_owned()
    }

    fn tar_listing(tarball: &Path) -> Vec<String> {
        let output = Command::new("tar")
            .args(["-tzf", &tarball.to_string_lossy()])
            .output()
            .unwrap();
        assert!(output.status.success(), "tar listing failed");
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_owned)
            .collect()
    }
}
