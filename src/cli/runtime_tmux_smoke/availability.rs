//! Shared tmux availability output for runtime smokes.

use crate::cli::CliExit;

pub(super) fn tmux_missing_skip_exit(smoke_name: &'static str) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "{smoke_name}: ok\ntmux available: false\nskipped: tmux not found on PATH\n"
        ),
        stderr: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::tmux_missing_skip_exit;

    #[test]
    fn runtime_tmux_smoke_missing_tmux_skip_is_clean_and_grep_friendly() {
        let exit = tmux_missing_skip_exit("runtime tmux smoke");

        assert_eq!(exit.code, 0);
        assert_eq!(
            exit.stdout,
            "runtime tmux smoke: ok\ntmux available: false\nskipped: tmux not found on PATH\n"
        );
        assert!(exit.stderr.is_empty());
    }

    #[test]
    fn runtime_tmux_ui_smoke_missing_tmux_skip_is_clean_and_grep_friendly() {
        let exit = tmux_missing_skip_exit("runtime tmux ui smoke");

        assert_eq!(exit.code, 0);
        assert_eq!(
            exit.stdout,
            "runtime tmux ui smoke: ok\ntmux available: false\nskipped: tmux not found on PATH\n"
        );
        assert!(exit.stderr.is_empty());
    }
}
