//! Required-check reporting for the base tmux runtime smoke.

use crate::cli::CliExit;

pub(super) fn runtime_tmux_smoke_result(stdout: String) -> CliExit {
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
        stdout: stdout.replacen("runtime tmux smoke: ok", "runtime tmux smoke: failed", 1),
        stderr: format!(
            "runtime tmux smoke failed: required checks failed: {}\n",
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
    if value == "false" {
        return Some(label);
    }
    if matches!(label, "state sessions" | "state windows" | "state panes") && value == "0" {
        return Some(label);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::runtime_tmux_smoke_result;

    #[test]
    fn runtime_tmux_smoke_report_fails_when_required_check_is_false() {
        let exit = runtime_tmux_smoke_result(
            "runtime tmux smoke: ok\ncreated session: true\nstate reader observed session: false\ncleanup killed session: true\n".to_owned(),
        );

        assert_eq!(exit.code, 1);
        assert!(
            exit.stdout
                .starts_with("runtime tmux smoke: failed\ncreated session: true")
        );
        assert!(
            exit.stderr
                .contains("required checks failed: state reader observed session")
        );
    }

    #[test]
    fn runtime_tmux_smoke_report_fails_when_required_state_count_is_zero() {
        let exit = runtime_tmux_smoke_result(
            "runtime tmux smoke: ok\nstate sessions: 1\nstate windows: 0\nstate panes: 3\n"
                .to_owned(),
        );

        assert_eq!(exit.code, 1);
        assert!(
            exit.stderr
                .contains("required checks failed: state windows")
        );
    }

    #[test]
    fn runtime_tmux_smoke_report_keeps_success_when_required_checks_pass() {
        let exit = runtime_tmux_smoke_result(
            "runtime tmux smoke: ok\ncreated session: true\nstate sessions: 1\nstate windows: 2\nstate panes: 3\ncleanup killed session: true\n".to_owned(),
        );

        assert_eq!(exit.code, 0);
        assert!(exit.stdout.starts_with("runtime tmux smoke: ok"));
        assert!(exit.stderr.is_empty());
    }
}
