use std::cell::RefCell;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{MockBackend, run_with_backend};

#[test]
fn theme_list_cli_reports_builtin_theme_tokens_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--theme-list"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("theme presets:"));
    assert!(exit.stdout.contains("- gromaq-ghostty default"));
    assert!(exit.stdout.contains("- gromaq-dark"));
    assert!(exit.stdout.contains("- gromaq-graphite"));
    assert!(exit.stdout.contains("background: #101216"));
    assert!(exit.stdout.contains("foreground: #eef4fb"));
    assert!(exit.stdout.contains("background opacity: 1"));
    assert!(exit.stdout.contains("surface padding px: 14"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn theme_list_cli_rejects_extra_arguments() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--theme-list", "extra"], &backend);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.starts_with("usage: gromaq ["));
    assert!(exit.stderr.contains("unexpected extra argument: extra"));
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn theme_legibility_smoke_reports_default_visual_gates_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--theme-legibility-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("theme legibility smoke: ok"));
    assert!(exit.stdout.contains("preset: gromaq-ghostty"));
    assert!(exit.stdout.contains("font size px: 34"));
    assert!(exit.stdout.contains("cell width px: 19"));
    assert!(exit.stdout.contains("line height px: 47"));
    assert!(exit.stdout.contains("background opacity percent: 100"));
    assert!(exit.stdout.contains("foreground/background contrast x100:"));
    assert!(exit.stdout.contains("foreground/selection contrast x100:"));
    assert!(exit.stdout.contains("cursor/background contrast x100:"));
    assert!(exit.stdout.contains("readable ansi min contrast x100:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

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
