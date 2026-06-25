use std::cell::RefCell;

use super::super::{MockBackend, run_with_backend};

#[test]
fn theme_legibility_smoke_reports_default_visual_gates_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--theme-legibility-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("theme legibility smoke: ok"));
    assert!(exit.stdout.contains("preset: gromaq-ghostty"));
    assert!(exit.stdout.contains("font size px: 32"));
    assert!(exit.stdout.contains("cell width px: 18"));
    assert!(exit.stdout.contains("line height px: 44"));
    assert!(exit.stdout.contains("background opacity percent: 100"));
    assert!(exit.stdout.contains("foreground/background contrast x100:"));
    assert!(exit.stdout.contains("foreground/selection contrast x100:"));
    assert!(exit.stdout.contains("cursor/background contrast x100:"));
    assert!(exit.stdout.contains("readable ansi min contrast x100:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}
