use crate::support::*;

#[test]
fn pty_session_runs_bash_command_when_available() {
    assert_shell_command_outputs("bash", "gromaq-bash");
}

#[test]
fn pty_session_runs_bash_interactive_workflow_when_available() {
    assert_interactive_shell_outputs_when_available(
        "bash",
        b"printf 'gromaq-bash-interactive\\n'\nexit\n",
        "gromaq-bash-interactive",
    );
}

#[test]
fn pty_session_runs_zsh_command_when_available() {
    assert_shell_command_outputs("zsh", "gromaq-zsh");
}

#[test]
fn pty_session_runs_zsh_interactive_workflow_when_available() {
    assert_interactive_shell_outputs_when_available(
        "zsh",
        b"printf 'gromaq-zsh-interactive\\n'\nexit\n",
        "gromaq-zsh-interactive",
    );
}

#[test]
fn pty_session_runs_fish_command_when_available() {
    assert_program_outputs_when_available("fish", &["-c", "printf gromaq-fish"], "gromaq-fish");
}

#[test]
fn pty_session_runs_fish_interactive_workflow_when_available() {
    assert_interactive_shell_outputs_when_available(
        "fish",
        b"printf 'gromaq-fish-interactive\\n'\nexit\n",
        "gromaq-fish-interactive",
    );
}

#[test]
fn pty_session_runs_nushell_command_when_available() {
    assert_program_outputs_when_available("nu", &["-c", "print gromaq-nushell"], "gromaq-nushell");
}

#[test]
fn pty_session_runs_nushell_interactive_workflow_when_available() {
    assert_interactive_shell_outputs_when_available(
        "nu",
        b"print gromaq-nushell-interactive\nexit\n",
        "gromaq-nushell-interactive",
    );
}
