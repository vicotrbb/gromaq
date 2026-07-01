//! Native tmux manager CLI diagnostics.

use super::CliExit;
use crate::tmux::{
    SystemTmuxCommandRunner, TmuxAction, TmuxError, TmuxManager, TmuxManagerSnapshot, TmuxProbe,
    TmuxState, shell_quote,
};

/// Report native tmux manager state without mutating tmux.
pub(super) fn tmux_manager_exit() -> CliExit {
    let runner = SystemTmuxCommandRunner;
    let probe = match TmuxProbe::new(runner).probe() {
        Ok(probe) => probe,
        Err(error) => return error_exit(error),
    };
    if !probe.installed {
        return CliExit {
            code: 0,
            stdout: render_state(false, None, &TmuxState::default()),
            stderr: String::new(),
        };
    }
    match TmuxManager::new(runner).snapshot() {
        Ok(snapshot) => CliExit {
            code: 0,
            stdout: render_snapshot(snapshot),
            stderr: String::new(),
        },
        Err(TmuxError::Command(_)) => CliExit {
            code: 0,
            stdout: render_state(true, None, &TmuxState::default()),
            stderr: String::new(),
        },
        Err(error) => error_exit(error),
    }
}

fn render_snapshot(snapshot: TmuxManagerSnapshot) -> String {
    let current = snapshot.current.as_ref().map(|current| {
        format!(
            "{}:{}:{}",
            current.session_name, current.window_index, current.pane_id
        )
    });
    let mut output = render_state(true, current.as_deref(), &snapshot.state);
    if let Some(session) = snapshot.current_session() {
        output.push_str(&format!("current session: {}\n", session.name));
    }
    for window in snapshot.current_windows() {
        output.push_str(&format!(
            "current session window: {}:{} {}\n",
            window.session_name, window.index, window.name
        ));
    }
    for pane in snapshot.current_window_panes() {
        output.push_str(&format!(
            "current window pane: {}:{}:{} {}\n",
            pane.session_name, pane.window_index, pane.id, pane.current_command
        ));
    }
    output
}

fn render_state(installed: bool, current: Option<&str>, state: &TmuxState) -> String {
    let mut output = format!(
        "tmux manager\ntmux installed: {installed}\nsessions: {}\nwindows: {}\npanes: {}\n",
        state.sessions.len(),
        state.windows.len(),
        state.panes.len()
    );
    if let Some(current) = current {
        output.push_str(&format!("current: {current}\n"));
    }
    output.push_str(&status_guidance(installed, current, state));
    for session in &state.sessions {
        output.push_str(&format!(
            "session: {} attached={}\n",
            session.name, session.attached
        ));
    }
    for window in &state.windows {
        output.push_str(&format!(
            "window: {}:{} {} active={}\n",
            window.session_name, window.index, window.name, window.active
        ));
    }
    for pane in &state.panes {
        output.push_str(&format!(
            "pane: {}:{}:{} {} command={} active={}\n",
            pane.session_name,
            pane.window_index,
            pane.id,
            pane.title,
            pane.current_command,
            pane.active
        ));
    }
    render_manager_actions(&mut output);
    output
}

fn status_guidance(installed: bool, current: Option<&str>, state: &TmuxState) -> String {
    if !installed {
        return "status: missing\nnext: install tmux or disable tmux workflows\n".to_owned();
    }
    if current.is_some() {
        return "status: attached\n".to_owned();
    }
    if state.sessions.is_empty() {
        return "status: no server\nnext: run gromaq --tmux-action start-session <session>\n"
            .to_owned();
    }
    format!(
        "status: detached\nnext: run gromaq --tmux-action attach-session {}\n",
        shell_quote(&state.sessions[0].name)
    )
}

fn render_manager_actions(output: &mut String) {
    for action in TmuxAction::registry() {
        output.push_str(&format!(
            "manager action: {}\n  label: {}\n  run: gromaq --tmux-action {}\n  tmux command: {}\n",
            action.stable_id, action.label, action.stable_id, action.tmux_command
        ));
        if let Some(key) = action.key_binding {
            output.push_str(&format!("  tmux key: {key}\n"));
        }
        output.push_str(&format!(
            "  destructive: {}\n  confirmation required: {}\n",
            action.destructive, action.confirmation_required
        ));
    }
}

fn error_exit(error: TmuxError) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("tmux manager failed: {error:?}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::render_state;
    use crate::tmux::{TmuxSession, TmuxState};

    #[test]
    fn tmux_manager_report_teaches_no_server_next_step() {
        let report = render_state(true, None, &TmuxState::default());

        assert!(report.contains("status: no server"));
        assert!(report.contains("next: run gromaq --tmux-action start-session <session>"));
    }

    #[test]
    fn tmux_manager_report_teaches_detached_session_next_step() {
        let report = render_state(
            true,
            None,
            &TmuxState {
                sessions: vec![TmuxSession {
                    name: "delta work".to_owned(),
                    attached: false,
                }],
                ..TmuxState::default()
            },
        );

        assert!(report.contains("status: detached"));
        assert!(report.contains("next: run gromaq --tmux-action attach-session 'delta work'"));
    }
}
