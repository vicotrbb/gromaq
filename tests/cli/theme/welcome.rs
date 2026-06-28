use std::cell::RefCell;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::{MockBackend, run_with_backend};

#[test]
fn welcome_preview_snapshot_writes_default_welcome_ppm_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = std::env::temp_dir().join(format!(
        "gromaq-welcome-preview-{}.ppm",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let exit = run_with_backend(
        [
            "gromaq",
            "--welcome-preview-snapshot",
            path.to_str().unwrap(),
        ],
        &backend,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("welcome preview snapshot: ok"));
    assert!(exit.stdout.contains("preset: gromaq-ghostty"));
    assert!(exit.stdout.contains("terminal cells: 80x18"));
    assert!(exit.stdout.contains("high contrast text pixels:"));
    assert!(exit.stdout.contains("avatar color pixels:"));
    assert!(exit.stdout.contains("glyph quads:"));
    assert!(exit.stdout.contains("cursor quads: 0"));
    assert!(exit.stdout.contains("atlas bytes:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());

    let snapshot = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(snapshot.starts_with(b"P6\n"));
    assert!(snapshot.len() > 1024);
}

#[test]
fn welcome_preview_snapshot_requires_output_path() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--welcome-preview-snapshot"], &backend);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.starts_with("usage: gromaq ["));
    assert!(
        exit.stderr
            .contains("missing snapshot path for --welcome-preview-snapshot")
    );
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn welcome_preview_snapshot_rejects_extra_arguments() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = std::env::temp_dir().join("gromaq-welcome-preview-extra.ppm");

    let exit = run_with_backend(
        [
            "gromaq",
            "--welcome-preview-snapshot",
            path.to_str().unwrap(),
            "extra",
        ],
        &backend,
    );

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.starts_with("usage: gromaq ["));
    assert!(exit.stderr.contains("unexpected extra argument: extra"));
    assert!(backend.requests.borrow().is_empty());
    assert!(!path.exists());
}
