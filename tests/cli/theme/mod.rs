use std::time::{SystemTime, UNIX_EPOCH};

mod export;
mod legibility;
mod list;
mod preview;

fn temp_theme_path(prefix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}-{}.toml",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
