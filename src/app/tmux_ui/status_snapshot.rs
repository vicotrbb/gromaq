use crate::app::{TmuxStatusKind, TmuxUiSnapshot};
use crate::tmux::{TmuxManagerSnapshot, TmuxManagerStatus};

impl TmuxUiSnapshot {
    /// Build a status-strip snapshot from a native tmux manager snapshot.
    pub fn from_manager_snapshot(snapshot: &TmuxManagerSnapshot) -> Self {
        let current_panes = snapshot.current_window_panes();
        let active_pane = snapshot
            .current
            .as_ref()
            .and_then(|current| {
                snapshot
                    .state
                    .panes
                    .iter()
                    .find(|pane| pane.id == current.pane_id)
            })
            .or_else(|| current_panes.iter().copied().find(|pane| pane.active));
        Self {
            status: status_kind(snapshot),
            current_session: current_session(snapshot),
            current_window: current_window_label(snapshot),
            visible_windows: visible_windows(snapshot),
            pane_count: current_pane_count(&current_panes),
            active_pane_id: active_pane.map(|pane| pane.id.clone()),
            active_pane_command: active_pane.map(|pane| pane.current_command.clone()),
            pending_feedback: None,
            confirmation_feedback: None,
        }
    }
}

fn status_kind(snapshot: &TmuxManagerSnapshot) -> TmuxStatusKind {
    match snapshot.status {
        TmuxManagerStatus::Missing => TmuxStatusKind::Missing,
        TmuxManagerStatus::NoServer => TmuxStatusKind::NoServer,
        TmuxManagerStatus::Available if snapshot.current.is_some() => TmuxStatusKind::Attached,
        TmuxManagerStatus::Available if snapshot.state.sessions.is_empty() => {
            TmuxStatusKind::NoServer
        }
        TmuxManagerStatus::Available => TmuxStatusKind::Detached,
    }
}

fn current_session(snapshot: &TmuxManagerSnapshot) -> Option<String> {
    snapshot
        .current
        .as_ref()
        .map(|current| current.session_name.clone())
        .or_else(|| {
            snapshot
                .state
                .sessions
                .first()
                .map(|session| session.name.clone())
        })
}

fn current_window_label(snapshot: &TmuxManagerSnapshot) -> Option<String> {
    let current = snapshot.current.as_ref()?;
    snapshot
        .state
        .windows
        .iter()
        .find(|window| {
            window.session_name == current.session_name && window.index == current.window_index
        })
        .map(|window| format!("{}:{}", window.index, window.name))
        .or_else(|| Some(current.window_index.to_string()))
}

fn visible_windows(snapshot: &TmuxManagerSnapshot) -> Vec<String> {
    snapshot
        .current_windows()
        .into_iter()
        .map(|window| {
            let active = if window.active { "*" } else { "" };
            format!("{}:{}{active}", window.index, window.name)
        })
        .collect()
}

fn current_pane_count(panes: &[&crate::tmux::TmuxPane]) -> Option<usize> {
    if panes.is_empty() {
        None
    } else {
        Some(panes.len())
    }
}
