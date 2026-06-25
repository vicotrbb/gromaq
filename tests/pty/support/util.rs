use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn top_snapshot_args() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["-l", "1", "-n", "5"]
    } else {
        &["-b", "-n", "1"]
    }
}

pub(crate) fn find_program(program: &str) -> Option<OsString> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(program))
        .find(|candidate| is_executable_file(candidate.as_path()))
        .map(OsString::from)
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

pub(crate) fn test_temp_path(name: &str) -> PathBuf {
    let directory = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("gromaq-pty-tests");
    fs::create_dir_all(&directory).unwrap();
    directory.join(format!("{}-{name}", std::process::id()))
}

pub(crate) fn shell_quote_path(path: &Path) -> String {
    shell_quote(&path.to_string_lossy())
}

pub(crate) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
