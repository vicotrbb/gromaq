use std::cell::RefCell;

use crate::{MockBackend, run_with_backend};

#[test]
fn runtime_perf_smoke_cli_reports_structured_metrics_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-perf-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime perf smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes: 1"));
    assert!(exit.stdout.contains("rendered frames: 1"));
    assert!(exit.stdout.contains("rendered dirty regions:"));
    assert!(exit.stdout.contains("rendered dirty cells:"));
    assert!(exit.stdout.contains("rendered dirty cells max:"));
    assert!(exit.stdout.contains("render samples: 1"));
    assert!(exit.stdout.contains("render avg ns:"));
    assert!(exit.stdout.contains("render max ns:"));
    assert!(exit.stdout.contains("render p95 ns:"));
    assert!(exit.stdout.contains("input-to-render samples: 1"));
    assert!(exit.stdout.contains("input-to-render avg ns:"));
    assert!(exit.stdout.contains("input-to-render max ns:"));
    assert!(exit.stdout.contains("input-to-render p95 ns:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_perf_budget_smoke_cli_reports_repeated_budget_samples_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-perf-budget-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime perf budget smoke: ok"));
    assert!(exit.stdout.contains("samples: 20"));
    assert!(exit.stdout.contains("pumped bytes: 20"));
    assert!(exit.stdout.contains("rendered frames: 20"));
    assert!(exit.stdout.contains("render p95 ns:"));
    assert!(exit.stdout.contains("render p95 budget ns: 6940000"));
    assert!(exit.stdout.contains("input-to-render p95 ns:"));
    assert!(
        exit.stdout
            .contains("input-to-render p95 budget ns: 10000000")
    );
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_perf_p95_smoke_cli_reports_repeated_budget_metrics_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-perf-p95-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime perf p95 smoke: ok"));
    assert!(exit.stdout.contains("samples: 20"));
    assert!(exit.stdout.contains("pumped bytes: 20"));
    assert!(exit.stdout.contains("rendered frames: 20"));
    assert!(exit.stdout.contains("render p95 ns:"));
    assert!(exit.stdout.contains("render p95 budget ns: 6940000"));
    assert!(exit.stdout.contains("input-to-render p95 ns:"));
    assert!(
        exit.stdout
            .contains("input-to-render p95 budget ns: 10000000")
    );
    assert!(exit.stdout.contains("render max ns:"));
    assert!(exit.stdout.contains("input-to-render max ns:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}
