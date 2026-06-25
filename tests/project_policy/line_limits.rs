use std::path::Path;

use super::support::collect_line_limit_violations;

const MAX_SOURCE_FILE_LINES: usize = 214;
const MAX_INTEGRATION_TEST_FILE_LINES: usize = 360;
const MAX_CLI_TEST_FILE_LINES: usize = 285;
const MAX_BENCHMARK_FILE_LINES: usize = 315;

#[test]
fn source_modules_stay_small_enough_to_review() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = root.join("src");
    let mut violations = Vec::new();

    collect_line_limit_violations(root, &src, MAX_SOURCE_FILE_LINES, &mut violations);
    violations.sort();

    assert!(
        violations.is_empty(),
        "source files must stay under {MAX_SOURCE_FILE_LINES} lines for reviewable module boundaries: {violations:#?}"
    );
}

#[test]
fn integration_test_modules_stay_small_enough_to_review() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tests = root.join("tests");
    let mut violations = Vec::new();

    collect_line_limit_violations(
        root,
        &tests,
        MAX_INTEGRATION_TEST_FILE_LINES,
        &mut violations,
    );
    violations.sort();

    assert!(
        violations.is_empty(),
        "integration test files must stay under {MAX_INTEGRATION_TEST_FILE_LINES} lines for reviewable behavior groups: {violations:#?}"
    );
}

#[test]
fn cli_test_modules_stay_small_enough_to_review() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cli_tests = root.join("tests/cli");
    let mut violations = Vec::new();

    collect_line_limit_violations(root, &cli_tests, MAX_CLI_TEST_FILE_LINES, &mut violations);
    violations.sort();

    assert!(
        violations.is_empty(),
        "CLI test files must stay under {MAX_CLI_TEST_FILE_LINES} lines for reviewable behavior groups: {violations:#?}"
    );
}

#[test]
fn benchmark_modules_stay_small_enough_to_review() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let benches = root.join("benches");
    let mut violations = Vec::new();

    collect_line_limit_violations(root, &benches, MAX_BENCHMARK_FILE_LINES, &mut violations);
    violations.sort();

    assert!(
        violations.is_empty(),
        "benchmark files must stay under {MAX_BENCHMARK_FILE_LINES} lines for reviewable measurement groups: {violations:#?}"
    );
}
