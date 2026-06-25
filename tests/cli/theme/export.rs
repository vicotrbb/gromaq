use std::cell::RefCell;
use std::fs;

use gromaq::{GromaqConfig, ThemePresetSetting, ThemeSettings};

use super::super::{MockBackend, run_with_backend};
use super::temp_theme_path;

#[test]
fn theme_export_cli_writes_parseable_theme_toml_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = temp_theme_path("gromaq-theme-export");

    let exit = run_with_backend(
        [
            "gromaq",
            "--theme-export",
            "gromaq-graphite",
            path.to_str().unwrap(),
        ],
        &backend,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("theme export: ok"));
    assert!(exit.stdout.contains("preset: gromaq-graphite"));
    assert!(exit.stdout.contains("bytes written:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());

    let exported = fs::read_to_string(&path).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(exported.starts_with("[theme]\n"));
    assert!(exported.contains("preset = \"gromaq-graphite\""));
    assert!(exported.contains("background_opacity = 1"));
    assert!(exported.contains("cursor_opacity = 1"));
    assert!(exported.contains("selection_opacity = 1"));
    let parsed = GromaqConfig::from_toml_str(&exported).unwrap();
    assert_eq!(
        parsed.theme,
        ThemeSettings::from_preset(ThemePresetSetting::GromaqGraphite)
    );
}

#[test]
fn theme_export_cli_rejects_unknown_preset_without_writing_file() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = temp_theme_path("gromaq-theme-export-invalid");

    let exit = run_with_backend(
        [
            "gromaq",
            "--theme-export",
            "ghostty",
            path.to_str().unwrap(),
        ],
        &backend,
    );

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.contains("theme export failed:"));
    assert!(exit.stderr.contains("unknown theme preset `ghostty`"));
    assert!(backend.requests.borrow().is_empty());
    assert!(!path.exists());
}

#[test]
fn theme_export_cli_requires_preset_and_path() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let missing_preset = run_with_backend(["gromaq", "--theme-export"], &backend);
    assert_eq!(missing_preset.code, 2);
    assert!(missing_preset.stdout.is_empty());
    assert!(
        missing_preset
            .stderr
            .contains("missing theme preset for --theme-export")
    );

    let missing_path = run_with_backend(["gromaq", "--theme-export", "gromaq-ghostty"], &backend);
    assert_eq!(missing_path.code, 2);
    assert!(missing_path.stdout.is_empty());
    assert!(
        missing_path
            .stderr
            .contains("missing export path for --theme-export")
    );
    assert!(backend.requests.borrow().is_empty());
}
