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
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

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

    #[test]
    fn debian_package_script_assembles_installable_desktop_package_from_stub_binary() {
        let dist = run_packaging_script(
            "scripts/package-debian-deb.sh",
            &[("GROMAQ_DEB_ARCH", "amd64")],
        );
        let deb = single_deb(dist.path());
        let members = ar_member_names(&deb);
        assert_eq!(
            members,
            ["debian-binary", "control.tar.gz", "data.tar.gz"],
            "deb archive must contain canonical members"
        );

        let control_tar = extract_ar_member(&deb, "control.tar.gz", dist.path());
        let control = tar_file_contents(&control_tar, "./control");
        for required in [
            "Package: gromaq",
            "Version: 0.1.0",
            "Architecture: amd64",
            "Maintainer: Gromaq contributors",
            "Description: Native Rust GPU-rendered terminal emulator foundation for gromaq.dev",
        ] {
            assert!(control.contains(required), "control missing {required}");
        }

        let data_tar = extract_ar_member(&deb, "data.tar.gz", dist.path());
        let listing = tar_listing(&data_tar);
        for required in [
            "./usr/bin/gromaq",
            "./usr/share/doc/gromaq/README.md",
            "./usr/share/doc/gromaq/copyright",
            "./usr/share/applications/dev.gromaq.Gromaq.desktop",
            "./usr/share/metainfo/dev.gromaq.Gromaq.metainfo.xml",
            "./usr/share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png",
        ] {
            assert!(
                listing.iter().any(|entry| entry == required),
                "deb data missing {required}; listing:\n{}",
                listing.join("\n")
            );
        }
    }

    #[test]
    fn debian_package_script_accepts_relative_dist_dir() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let relative = format!("target/{}", unique_temp_name("gromaq-relative-deb"));
        let dist = RelativeTempDist(root.join(&relative));
        fs::create_dir_all(dist.path()).unwrap();
        let stub = dist.path().join("gromaq-stub");
        fs::write(&stub, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&stub, Permissions::from_mode(0o755)).unwrap();

        let output = Command::new("sh")
            .arg(root.join("scripts/package-debian-deb.sh"))
            .current_dir(root)
            .env("GROMAQ_BINARY_PATH", &stub)
            .env("GROMAQ_DEB_ARCH", "amd64")
            .env("GROMAQ_DIST_DIR", &relative)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "relative dist deb packaging failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        assert!(
            dist.path().join("gromaq_0.1.0_amd64.deb").is_file(),
            "relative dist package missing"
        );
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

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_app_script_can_codesign_bundle_when_identity_is_supplied() {
        let dist = run_packaging_script(
            "scripts/package-macos-app.sh",
            &[("GROMAQ_CODESIGN_IDENTITY", "-")],
        );
        let app = dist.path().join("Gromaq.app");

        let verify_ok = Command::new("codesign")
            .args(["--verify", "--deep", "--strict", &app.to_string_lossy()])
            .status()
            .unwrap()
            .success();
        assert!(
            verify_ok,
            "codesigned app bundle must pass strict verification"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn notarization_script_dry_run_prepares_archive_without_credentials() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let dist = TempDist(std::env::temp_dir().join(unique_temp_name("gromaq-notary")));
        let app = dist.path().join("Gromaq.app");
        let macos = app.join("Contents/MacOS");
        let zip = dist.path().join("Gromaq-notary.zip");
        fs::create_dir_all(&macos).unwrap();
        fs::write(macos.join("gromaq"), "#!/bin/sh\nexit 0\n").unwrap();

        let output = Command::new("sh")
            .arg(root.join("scripts/notarize-macos-app.sh"))
            .arg(&app)
            .env("GROMAQ_NOTARY_DRY_RUN", "1")
            .env("GROMAQ_NOTARY_ZIP_PATH", &zip)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "notarization dry run failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Dry run: prepared notarization archive"));
        assert!(stdout.contains("xcrun notarytool submit --wait"));
        assert!(stdout.contains("would staple and validate"));
        assert!(zip.is_file(), "dry run should create the notary zip");
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

    struct RelativeTempDist(PathBuf);

    impl RelativeTempDist {
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for RelativeTempDist {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn run_packaging_script(script: &str, extra_env: &[(&str, &str)]) -> TempDist {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let dist = TempDist(std::env::temp_dir().join(unique_temp_name("gromaq-packaging")));
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

    fn single_deb(dist: &Path) -> PathBuf {
        let mut packages: Vec<_> = fs::read_dir(dist)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "deb"))
            .collect();
        assert_eq!(
            packages.len(),
            1,
            "expected exactly one deb package: {packages:?}"
        );
        packages.pop().unwrap()
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

    fn ar_member_names(archive: &Path) -> Vec<String> {
        let output = Command::new("ar")
            .args(["-t", &archive.to_string_lossy()])
            .output()
            .unwrap();
        assert!(output.status.success(), "ar member listing failed");
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn extract_ar_member(archive: &Path, member: &str, dist: &Path) -> PathBuf {
        let output = Command::new("ar")
            .args(["-p", &archive.to_string_lossy(), member])
            .output()
            .unwrap();
        assert!(output.status.success(), "ar extraction failed for {member}");
        let path = dist.join(member);
        fs::write(&path, output.stdout).unwrap();
        path
    }

    fn tar_file_contents(tarball: &Path, path: &str) -> String {
        let output = Command::new("tar")
            .args(["-xOf", &tarball.to_string_lossy(), path])
            .output()
            .unwrap();
        assert!(output.status.success(), "tar extract failed for {path}");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}
