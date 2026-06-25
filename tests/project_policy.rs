use std::{collections::BTreeSet, fs, path::Path};

use toml::Value;

const FORBIDDEN_FRONTEND_FILES: &[&str] = &[
    "package.json",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lock",
    "bun.lockb",
    "vite.config.js",
    "vite.config.ts",
    "webpack.config.js",
    "webpack.config.ts",
    "electron-builder.json",
];

const FORBIDDEN_FRONTEND_EXTENSIONS: &[&str] =
    &["cjs", "cts", "js", "jsx", "mjs", "mts", "ts", "tsx"];

const FORBIDDEN_DEPENDENCIES: &[&str] = &[
    "boa_engine",
    "deno_core",
    "electron",
    "javascriptcore-rs",
    "nodejs-sys",
    "tauri",
    "web-view",
    "webbrowser",
    "webkit2gtk",
    "webview",
    "webview2-com",
    "wry",
];

const UNSAFE_FORBIDDEN_CRATE_ROOTS: &[&str] = &["src/lib.rs", "src/main.rs"];

const REQUIRED_REPOSITORY_FILES: &[&str] = &[
    "README.md",
    "ARCHITECTURE.md",
    "CONTRIBUTING.md",
    "BENCHMARKS.md",
    "COMPATIBILITY.md",
    "ROADMAP.md",
    "LICENSE",
    "TESTING.md",
    "DEBUGGING.md",
    "GOOD_FIRST_ISSUES.md",
    "documentation/benchmarks.md",
    "tests/fixtures/README.md",
    ".github/workflows/ci.yml",
    ".github/labels.yml",
    ".github/ISSUE_TEMPLATE/bug_report.md",
    ".github/ISSUE_TEMPLATE/compatibility_gap.md",
    ".github/ISSUE_TEMPLATE/performance_proof.md",
];

const REQUIRED_ISSUE_LABELS: &[&str] = &[
    "bug",
    "compatibility",
    "performance",
    "needs-proof",
    "needs-triage",
    "good first issue",
    "documentation",
    "tests",
    "gpu",
    "blocked-live-proof",
];

const REQUIRED_CI_COMMANDS: &[&str] = &[
    "cargo fmt --check",
    "git diff --check",
    "cargo clippy --all-targets --all-features -- -D warnings",
    "cargo test --all",
    "cargo run -- --theme-legibility-smoke",
    "cargo run -- --theme-preview-snapshot target/gromaq-theme-preview-ci.ppm",
    "cargo run -- --runtime-real-shell-perf-budget-smoke",
    "cargo run -- --runtime-real-shell-large-output-smoke",
    "cargo bench --bench parser_throughput -- --list",
];

#[test]
fn project_remains_native_rust_without_frontend_runtime_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();

    collect_frontend_file_violations(root, root, &mut violations);
    violations.sort();

    assert!(
        violations.is_empty(),
        "frontend runtime files are not allowed in native-only Gromaq: {violations:#?}"
    );
}

#[test]
fn repository_keeps_required_release_readiness_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for required_file in REQUIRED_REPOSITORY_FILES {
        let path = root.join(required_file);
        assert!(
            path.is_file(),
            "{required_file} must exist for repository release readiness"
        );
    }
}

#[test]
fn repository_keeps_required_issue_labels() {
    let labels_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/labels.yml");
    let labels = fs::read_to_string(&labels_path).unwrap();

    for label in REQUIRED_ISSUE_LABELS {
        let marker = format!("- name: {label}");
        assert!(
            labels.lines().any(|line| line.trim() == marker),
            "{} must define issue label `{label}`",
            relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), &labels_path)
        );
    }
}

#[test]
fn ci_workflow_runs_required_root_checks() {
    let workflow_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap();

    for command in REQUIRED_CI_COMMANDS {
        assert!(
            workflow.contains(command),
            "{} must run `{command}`",
            relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), &workflow_path)
        );
    }
}

#[test]
fn crate_roots_forbid_unsafe_code() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for crate_root in UNSAFE_FORBIDDEN_CRATE_ROOTS {
        let path = root.join(crate_root);
        let source = fs::read_to_string(&path).unwrap();
        assert!(
            source
                .lines()
                .any(|line| line.trim() == "#![forbid(unsafe_code)]"),
            "{crate_root} must keep #![forbid(unsafe_code)]"
        );
    }
}

#[test]
fn cargo_dependencies_do_not_add_webview_or_javascript_runtimes() {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    let manifest: Value = toml::from_str(&manifest).unwrap();
    let forbidden: BTreeSet<&str> = FORBIDDEN_DEPENDENCIES.iter().copied().collect();
    let mut violations = Vec::new();

    collect_forbidden_dependency_names(&manifest, &forbidden, &mut violations);
    violations.sort();
    violations.dedup();

    assert!(
        violations.is_empty(),
        "webview/browser/javascript runtime dependencies are not allowed: {violations:#?}"
    );
}

#[test]
fn cargo_manifest_keeps_public_open_source_metadata() {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    let manifest: Value = toml::from_str(&manifest).unwrap();
    let package = manifest
        .get("package")
        .and_then(Value::as_table)
        .expect("Cargo.toml must define [package]");

    assert_eq!(
        package.get("license").and_then(Value::as_str),
        Some("MIT"),
        "Cargo package metadata must publish the license"
    );
    assert_eq!(
        package.get("homepage").and_then(Value::as_str),
        Some("https://gromaq.dev"),
        "Cargo package metadata must keep the product homepage"
    );
    assert_eq!(
        package.get("repository").and_then(Value::as_str),
        Some("https://github.com/vicotrbb/gromaq"),
        "Cargo package metadata must point contributors at the public source repository"
    );
    assert_eq!(
        package.get("readme").and_then(Value::as_str),
        Some("README.md"),
        "Cargo package metadata must expose the README"
    );
    assert_string_array_contains(package, "keywords", "terminal");
    assert_string_array_contains(package, "keywords", "wgpu");
    assert_string_array_contains(package, "categories", "command-line-utilities");
}

fn collect_frontend_file_violations(root: &Path, dir: &Path, violations: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        if path.is_dir() {
            if matches!(file_name.as_ref(), ".git" | "target") {
                continue;
            }
            collect_frontend_file_violations(root, &path, violations);
            continue;
        }

        if is_forbidden_frontend_file(&path) {
            violations.push(relative_path(root, &path));
        }
    }
}

fn assert_string_array_contains(package: &toml::map::Map<String, Value>, field: &str, item: &str) {
    let values = package
        .get(field)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("Cargo package metadata must define `{field}`"));
    assert!(
        values.iter().any(|value| value.as_str() == Some(item)),
        "Cargo package metadata `{field}` must contain `{item}`"
    );
}

fn is_forbidden_frontend_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    if FORBIDDEN_FRONTEND_FILES.contains(&file_name) {
        return true;
    }

    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| FORBIDDEN_FRONTEND_EXTENSIONS.contains(&extension))
}

fn collect_forbidden_dependency_names(
    value: &Value,
    forbidden: &BTreeSet<&str>,
    violations: &mut Vec<String>,
) {
    let Some(table) = value.as_table() else {
        return;
    };

    for (key, value) in table {
        if is_dependency_table_name(key) {
            collect_dependency_table_violations(value, forbidden, violations);
        }
        collect_forbidden_dependency_names(value, forbidden, violations);
    }
}

fn is_dependency_table_name(key: &str) -> bool {
    matches!(
        key,
        "dependencies" | "dev-dependencies" | "build-dependencies"
    )
}

fn collect_dependency_table_violations(
    value: &Value,
    forbidden: &BTreeSet<&str>,
    violations: &mut Vec<String>,
) {
    let Some(table) = value.as_table() else {
        return;
    };

    for dependency in table.keys() {
        if forbidden.contains(dependency.as_str()) {
            violations.push(dependency.clone());
        }
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
