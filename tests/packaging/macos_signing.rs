use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn macos_app_script_ad_hoc_signs_and_verifies_by_default() {
    let dist = run_packaging_script();
    let app = dist.path().join("Gromaq.app");
    let summary = fs::read_to_string(dist.path().join("Gromaq-macos-app-summary.txt"))
        .expect("macOS app summary must be written");

    assert!(summary.contains("Codesign identity: -"));
    assert!(summary.contains("Codesign verification: strict"));
    let verify_ok = Command::new("codesign")
        .args([
            "--verify",
            "--deep",
            "--strict",
            "--verbose=4",
            &app.to_string_lossy(),
        ])
        .status()
        .unwrap()
        .success();
    assert!(
        verify_ok,
        "default packaged app must pass strict codesign verification"
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

fn run_packaging_script() -> TempDist {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dist = TempDist(std::env::temp_dir().join(unique_temp_name("gromaq-packaging")));
    let stub = dist.path().join("gromaq-stub");
    fs::create_dir_all(dist.path()).unwrap();
    fs::write(&stub, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&stub, Permissions::from_mode(0o755)).unwrap();

    let output = Command::new("sh")
        .arg(root.join("scripts/package-macos-app.sh"))
        .env("GROMAQ_BINARY_PATH", &stub)
        .env("GROMAQ_DIST_DIR", dist.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "package-macos-app.sh failed: {}",
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
