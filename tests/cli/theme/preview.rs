use std::cell::RefCell;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::{MockBackend, run_with_backend};

#[test]
fn theme_preview_snapshot_writes_default_theme_ppm_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = std::env::temp_dir().join(format!(
        "gromaq-theme-preview-{}.ppm",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let exit = run_with_backend(
        ["gromaq", "--theme-preview-snapshot", path.to_str().unwrap()],
        &backend,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("theme preview snapshot: ok"));
    assert!(exit.stdout.contains("preset: gromaq-ghostty"));
    assert!(exit.stdout.contains("font size px: 34"));
    assert!(exit.stdout.contains("cell width px: 19"));
    assert!(exit.stdout.contains("line height px: 47"));
    assert!(exit.stdout.contains("background opacity percent: 100"));
    assert!(exit.stdout.contains("surface padding px: 14"));
    assert!(exit.stdout.contains("cell spacing px: 0"));
    assert!(exit.stdout.contains("high contrast text pixels:"));
    assert!(exit.stdout.contains("selection pixels:"));
    assert!(exit.stdout.contains("cursor pixels:"));
    assert!(exit.stdout.contains("prepared quads:"));
    assert!(exit.stdout.contains("background quads:"));
    assert!(exit.stdout.contains("cursor quads:"));
    assert!(exit.stdout.contains("atlas bytes:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());

    let snapshot = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(snapshot.starts_with(b"P6\n"));
    assert!(snapshot.windows(4).any(|bytes| bytes == b"\n255"));
    assert!(snapshot.len() > 1024);
}

#[test]
fn theme_preview_config_writes_configured_theme_ppm_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let config_path = std::env::temp_dir().join(format!(
        "gromaq-theme-preview-config-{}.toml",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let snapshot_path = std::env::temp_dir().join(format!(
        "gromaq-theme-preview-config-{}.ppm",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &config_path,
        r##"
        [theme]
        preset = "gromaq-graphite"
        background_opacity = 0.75
        "##,
    )
    .unwrap();

    let exit = run_with_backend(
        [
            "gromaq",
            "--theme-preview-config",
            config_path.to_str().unwrap(),
            snapshot_path.to_str().unwrap(),
        ],
        &backend,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("theme preview snapshot: ok"));
    assert!(exit.stdout.contains("preset: gromaq-graphite"));
    assert!(exit.stdout.contains("background opacity percent: 75"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());

    let snapshot = fs::read(&snapshot_path).unwrap();
    fs::remove_file(&config_path).unwrap();
    fs::remove_file(&snapshot_path).unwrap();
    assert!(snapshot.starts_with(b"P6\n"));
    assert!(snapshot.len() > 1024);
}

#[test]
fn theme_preview_snapshot_requires_output_path() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--theme-preview-snapshot"], &backend);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.starts_with("usage: gromaq ["));
    assert!(
        exit.stderr
            .contains("missing snapshot path for --theme-preview-snapshot")
    );
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn theme_preview_config_requires_config_and_output_paths() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let missing_config = run_with_backend(["gromaq", "--theme-preview-config"], &backend);
    assert_eq!(missing_config.code, 2);
    assert!(missing_config.stdout.is_empty());
    assert!(
        missing_config
            .stderr
            .contains("missing config path for --theme-preview-config")
    );

    let missing_snapshot = run_with_backend(
        ["gromaq", "--theme-preview-config", "gromaq.toml"],
        &backend,
    );
    assert_eq!(missing_snapshot.code, 2);
    assert!(missing_snapshot.stdout.is_empty());
    assert!(
        missing_snapshot
            .stderr
            .contains("missing snapshot path for --theme-preview-config")
    );
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn theme_preview_config_rejects_invalid_config_without_writing_snapshot() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let config_path = std::env::temp_dir().join(format!(
        "gromaq-theme-preview-invalid-{}.toml",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let snapshot_path = std::env::temp_dir().join("gromaq-theme-preview-invalid.ppm");
    fs::write(&config_path, "[theme]\nbackground_opacity = 2.0\n").unwrap();

    let exit = run_with_backend(
        [
            "gromaq",
            "--theme-preview-config",
            config_path.to_str().unwrap(),
            snapshot_path.to_str().unwrap(),
        ],
        &backend,
    );

    fs::remove_file(&config_path).unwrap();
    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.contains("theme preview snapshot failed:"));
    assert!(exit.stderr.contains("background opacity"));
    assert!(backend.requests.borrow().is_empty());
    assert!(!snapshot_path.exists());
}

#[test]
fn theme_preview_config_rejects_extra_arguments() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let snapshot_path = std::env::temp_dir().join("gromaq-theme-preview-config-extra.ppm");

    let exit = run_with_backend(
        [
            "gromaq",
            "--theme-preview-config",
            "gromaq.toml",
            snapshot_path.to_str().unwrap(),
            "extra",
        ],
        &backend,
    );

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.starts_with("usage: gromaq ["));
    assert!(exit.stderr.contains("unexpected extra argument: extra"));
    assert!(backend.requests.borrow().is_empty());
    assert!(!snapshot_path.exists());
}

#[test]
fn theme_preview_snapshot_rejects_extra_arguments() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = std::env::temp_dir().join("gromaq-theme-preview-extra.ppm");

    let exit = run_with_backend(
        [
            "gromaq",
            "--theme-preview-snapshot",
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
