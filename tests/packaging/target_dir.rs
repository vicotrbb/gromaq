use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn linux_tarball_script_uses_cargo_target_dir_default_binary() {
    let proof = run_with_fake_cargo(
        "scripts/package-linux-tarball.sh",
        "gromaq-cargo-target-dir",
    );
    assert!(
        proof
            .dist
            .path()
            .join("gromaq-cargo-target-dir-0.2.1-linux-aarch64.tar.gz")
            .is_file(),
        "tarball should be assembled from the CARGO_TARGET_DIR release binary"
    );
}

#[test]
fn debian_package_script_uses_cargo_target_dir_default_binary() {
    let proof = run_with_fake_cargo(
        "scripts/package-debian-deb.sh",
        "gromaq-cargo-target-dir-deb",
    );
    assert!(
        proof
            .dist
            .path()
            .join("gromaq-cargo-target-dir-deb_0.2.1_arm64.deb")
            .is_file(),
        "deb package should be assembled from the CARGO_TARGET_DIR release binary"
    );
}

fn run_with_fake_cargo(script: &str, package: &str) -> ProofDirs {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dist = TempDir::new("gromaq-dist");
    let target = TempDir::new("gromaq-target-dir");
    let tools = TempDir::new("gromaq-tools");
    fs::create_dir_all(tools.path()).unwrap();
    let fake_cargo = tools.path().join("cargo");
    fs::write(
        &fake_cargo,
        format!(
            "#!/bin/sh\nset -eu\nmkdir -p \"$CARGO_TARGET_DIR/release\"\nprintf '#!/bin/sh\\nprintf \"{package} built from cargo target dir\\\\n\"\\n' > \"$CARGO_TARGET_DIR/release/{package}\"\nchmod 755 \"$CARGO_TARGET_DIR/release/{package}\"\n"
        ),
    )
    .unwrap();
    fs::set_permissions(&fake_cargo, Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        tools.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let output = Command::new("sh")
        .arg(root.join(script))
        .env("GROMAQ_PACKAGE", package)
        .env("GROMAQ_DIST_DIR", dist.path())
        .env("GROMAQ_RELEASE_TARGET", "linux-aarch64")
        .env("GROMAQ_DEB_ARCH", "arm64")
        .env("CARGO_TARGET_DIR", target.path())
        .env("PATH", path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{script} should package the CARGO_TARGET_DIR binary: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    ProofDirs {
        dist,
        _target: target,
        _tools: tools,
    }
}

struct ProofDirs {
    dist: TempDir,
    _target: TempDir,
    _tools: TempDir,
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(unique_temp_name(prefix));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
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
