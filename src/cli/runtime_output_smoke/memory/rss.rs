use std::process::Command;

pub(super) fn current_process_rss_kib() -> Result<u64, String> {
    let pid = std::process::id().to_string();
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .output()
        .map_err(|error| format!("process rss sampling failed to start: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "process rss sampling failed with status {}",
            output.status
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("process rss output was not utf-8: {error}"))?;
    stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| "process rss output was empty".to_owned())?
        .parse::<u64>()
        .map_err(|error| format!("process rss output was not numeric: {error}"))
}
