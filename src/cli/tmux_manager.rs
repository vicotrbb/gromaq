//! Native tmux manager CLI diagnostics.

use super::CliExit;
use crate::tmux::{
    SystemTmuxCommandRunner, TmuxAction, TmuxError, TmuxManager, TmuxManagerSnapshot, TmuxProbe,
    TmuxState,
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

fn render_manager_actions(output: &mut String) {
    for action in TmuxAction::registry() {
        output.push_str(&format!(
            "manager action: {}\n  label: {}\n  run: gromaq --tmux-action {}\n",
            action.stable_id, action.label, action.stable_id
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
