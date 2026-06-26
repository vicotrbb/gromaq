use std::{collections::BTreeSet, fs, path::Path};

use toml::Value;

use super::support::relative_path;

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

const ALLOWED_IMAGE_TOOLING_FILES: &[&str] = &[
    "images/avatar/generate.mjs",
    "images/logos/generate.mjs",
    "images/tools/gromaq-image-assets.mjs",
];

/// Directories that never contain Gromaq source and must be skipped by the
/// native-only scan: version-control metadata, build output, dependency trees,
/// and external tooling workspaces. `.opencode/` carries the opencode agent's
/// own node-based TUI state and is not part of the native Rust project, so it
/// must not be reported as a Gromaq frontend runtime file.
const NON_PROJECT_DIRECTORIES: &[&str] = &[".git", "target", "node_modules", ".opencode"];

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

fn collect_frontend_file_violations(root: &Path, dir: &Path, violations: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        if path.is_dir() {
            if NON_PROJECT_DIRECTORIES.contains(&file_name.as_ref()) {
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

fn is_forbidden_frontend_file(path: &Path) -> bool {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    if ALLOWED_IMAGE_TOOLING_FILES.contains(&relative_path(root, path).as_str()) {
        return false;
    }

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
