use std::cell::RefCell;

use gromaq::cli::run_with_backend;

use super::MockBackend;

#[test]
fn theme_legibility_smoke_reports_default_visual_gates_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--theme-legibility-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("theme legibility smoke: ok"));
    assert!(exit.stdout.contains("preset: gromaq-ghostty"));
    assert!(exit.stdout.contains("font size px: 37"));
    assert!(exit.stdout.contains("cell width px: 21"));
    assert!(exit.stdout.contains("line height px: 51"));
    assert!(exit.stdout.contains("foreground/background contrast x100:"));
    assert!(exit.stdout.contains("foreground/selection contrast x100:"));
    assert!(exit.stdout.contains("cursor/background contrast x100:"));
    assert!(exit.stdout.contains("readable ansi min contrast x100:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}
