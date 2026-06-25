use std::cell::RefCell;

use super::super::{MockBackend, run_with_backend};

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
