use crate::app::{TmuxStatusKind, TmuxUiSnapshot};
use crate::tmux::{TmuxManagerSnapshot, TmuxManagerStatus, TmuxPane, TmuxWindow};

impl TmuxUiSnapshot {
    /// Build a status-strip snapshot from a native tmux manager snapshot.
    pub fn from_manager_snapshot(snapshot: &TmuxManagerSnapshot) -> Self {
        let selected_window = selected_window(snapshot);
        let selected_panes = selected_window
            .map(|window| selected_window_panes(snapshot, window))
            .unwrap_or_default();
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
            .or_else(|| selected_panes.iter().copied().find(|pane| pane.active));
        Self {
            status: status_kind(snapshot),
            current_session: current_session(snapshot),
            current_window: current_window_label(snapshot, selected_window),
            visible_windows: visible_windows(snapshot),
            pane_count: current_pane_count(&selected_panes),
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
    selected_session_name(snapshot).map(str::to_owned)
}

fn selected_session_name(snapshot: &TmuxManagerSnapshot) -> Option<&str> {
    snapshot
        .current
        .as_ref()
        .map(|current| current.session_name.as_str())
        .or_else(|| {
            snapshot
                .state
                .sessions
                .first()
                .map(|session| session.name.as_str())
        })
}

fn selected_windows(snapshot: &TmuxManagerSnapshot) -> Vec<&TmuxWindow> {
    let Some(session_name) = selected_session_name(snapshot) else {
        return Vec::new();
    };
    snapshot
        .state
        .windows
        .iter()
        .filter(|window| window.session_name == session_name)
        .collect()
}

fn selected_window(snapshot: &TmuxManagerSnapshot) -> Option<&TmuxWindow> {
    if let Some(current) = snapshot.current.as_ref() {
        return snapshot.state.windows.iter().find(|window| {
            window.session_name == current.session_name && window.index == current.window_index
        });
    }
    selected_windows(snapshot)
        .into_iter()
        .find(|window| window.active)
}

fn current_window_label(
    snapshot: &TmuxManagerSnapshot,
    selected_window: Option<&TmuxWindow>,
) -> Option<String> {
    selected_window
        .map(|window| format!("{}:{}", window.index, window.name))
        .or_else(|| {
            snapshot
                .current
                .as_ref()
                .map(|current| current.window_index.to_string())
        })
}

fn selected_window_panes<'a>(
    snapshot: &'a TmuxManagerSnapshot,
    window: &TmuxWindow,
) -> Vec<&'a TmuxPane> {
    snapshot
        .state
        .panes
        .iter()
        .filter(|pane| {
            pane.session_name == window.session_name && pane.window_index == window.index
        })
        .collect()
}

fn visible_windows(snapshot: &TmuxManagerSnapshot) -> Vec<String> {
    selected_windows(snapshot)
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
