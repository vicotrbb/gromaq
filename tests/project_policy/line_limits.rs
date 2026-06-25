use std::path::Path;

use super::support::collect_line_limit_violations;

const MAX_SOURCE_FILE_LINES: usize = 214;
const MAX_CLI_TEST_FILE_LINES: usize = 285;

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
