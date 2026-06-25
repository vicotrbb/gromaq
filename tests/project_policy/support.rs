use std::{fs, path::Path};

pub fn collect_line_limit_violations(
    root: &Path,
    dir: &Path,
    maximum_lines: usize,
    violations: &mut Vec<String>,
) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            collect_line_limit_violations(root, &path, maximum_lines, violations);
            continue;
        }

        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }

        let source = fs::read_to_string(&path).unwrap();
        let line_count = source.lines().count();
        if line_count > maximum_lines {
            violations.push(format!(
                "{} has {line_count} lines",
                relative_path(root, &path)
            ));
        }
    }
}

pub fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
