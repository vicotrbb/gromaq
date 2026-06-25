use std::cell::RefCell;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{MockBackend, run_with_backend};

#[test]
fn runtime_glyph_frame_smoke_cli_reports_prepared_frame_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-glyph-frame-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime glyph frame smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes: 19"));
    assert!(exit.stdout.contains("planned glyphs:"));
    assert!(exit.stdout.contains("selection backgrounds:"));
    assert!(exit.stdout.contains("renderer atlas hits:"));
    assert!(exit.stdout.contains("renderer atlas misses:"));
    assert!(exit.stdout.contains("renderer atlas entries:"));
    assert!(exit.stdout.contains("rasterized glyphs:"));
    assert!(exit.stdout.contains("prepared quads:"));
    assert!(exit.stdout.contains("background quads:"));
    assert!(exit.stdout.contains("cursor quads:"));
    assert!(exit.stdout.contains("atlas bytes:"));
    assert!(exit.stdout.contains("frame size:"));
    assert!(exit.stdout.contains("line height px: 47"));
    assert!(exit.stdout.contains("surface padding px: 14"));
    assert!(exit.stdout.contains("cell spacing px: 0"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_glyph_frame_snapshot_cli_writes_preview_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = std::env::temp_dir().join(format!(
        "gromaq-runtime-glyph-frame-{}.ppm",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let exit = run_with_backend(
        [
            "gromaq",
            "--runtime-glyph-frame-snapshot",
            path.to_str().unwrap(),
        ],
        &backend,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime glyph frame snapshot: ok"));
    assert!(exit.stdout.contains("bytes written:"));
    assert!(exit.stdout.contains("frame size:"));
    assert!(exit.stdout.contains("prepared quads:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());

    let snapshot = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(snapshot.starts_with(b"P6\n"));
    assert!(snapshot.windows(4).any(|bytes| bytes == b"\n255"));
    assert!(snapshot.len() > 128);
}

#[test]
fn runtime_glyph_frame_snapshot_cli_requires_output_path() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-glyph-frame-snapshot"], &backend);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.starts_with("usage: gromaq ["));
    assert!(
        exit.stderr
            .contains("missing snapshot path for --runtime-glyph-frame-snapshot")
    );
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_glyph_frame_snapshot_cli_rejects_extra_arguments() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = std::env::temp_dir().join(format!(
        "gromaq-runtime-glyph-frame-extra-{}.ppm",
        std::process::id()
    ));
    let _ = fs::remove_file(&path);

    let exit = run_with_backend(
        [
            "gromaq",
            "--runtime-glyph-frame-snapshot",
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
