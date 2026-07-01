//! Required-check reporting for the native tmux UI smoke.

use crate::cli::CliExit;

pub(super) fn runtime_tmux_ui_smoke_result(stdout: String) -> CliExit {
    let failed = failed_check_labels(&stdout);
    if failed.is_empty() {
        return CliExit {
            code: 0,
            stdout,
            stderr: String::new(),
        };
    }

    CliExit {
        code: 1,
        stdout: stdout.replacen(
            "runtime tmux ui smoke: ok",
            "runtime tmux ui smoke: failed",
            1,
        ),
        stderr: format!(
            "runtime tmux ui smoke failed: required checks failed: {}\n",
            failed.join(", ")
        ),
    }
}

fn failed_check_labels(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(required_check_failure)
        .map(str::to_owned)
        .collect()
}

fn required_check_failure(line: &str) -> Option<&str> {
    let (label, value) = line.split_once(": ")?;
    if value == "false" || value.contains("=false") || value == "not-started" {
        return Some(label);
    }
    if matches!(label, "state sessions" | "state windows" | "state panes") && value == "0" {
        return Some(label);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::runtime_tmux_ui_smoke_result;

    #[test]
    fn ui_smoke_report_fails_when_required_check_is_false() {
        let exit = runtime_tmux_ui_smoke_result(
            "runtime tmux ui smoke: ok\nmanager panel opened: false\ncleanup killed session: true\n"
                .to_owned(),
        );

        assert_eq!(exit.code, 1);
        assert!(
            exit.stdout
                .starts_with("runtime tmux ui smoke: failed\nmanager panel opened: false")
        );
        assert!(
            exit.stderr
                .contains("required checks failed: manager panel opened")
        );
    }

    #[test]
    fn ui_smoke_report_fails_on_compound_handoff_and_state_checks() {
        let exit = runtime_tmux_ui_smoke_result(
            "runtime tmux ui smoke: ok\nskipped pty handoffs checked: attach=true start=false workspace=true\nstate sessions: 0\n".to_owned(),
        );

        assert_eq!(exit.code, 1);
        assert!(
            exit.stderr
                .contains("skipped pty handoffs checked, state sessions")
        );
    }

    #[test]
    fn ui_smoke_report_keeps_success_when_required_checks_pass() {
        let exit = runtime_tmux_ui_smoke_result(
            "runtime tmux ui smoke: ok\nmanager panel opened: true\nstate sessions: 1\n".to_owned(),
        );

        assert_eq!(exit.code, 0);
        assert!(exit.stdout.starts_with("runtime tmux ui smoke: ok"));
        assert!(exit.stderr.is_empty());
    }
}
