//! Native tmux manager selection helpers.

use crate::tmux::{TmuxManagerSnapshot, TmuxPane, TmuxWindow};

pub(super) fn selected_session_index(snapshot: &TmuxManagerSnapshot) -> usize {
    let Some(current) = snapshot.current.as_ref() else {
        return 0;
    };
    snapshot
        .state
        .sessions
        .iter()
        .position(|session| session.name == current.session_name)
        .unwrap_or(0)
}

pub(super) fn selected_window_index(snapshot: &TmuxManagerSnapshot, session_index: usize) -> usize {
    let Some(current) = snapshot.current.as_ref() else {
        return 0;
    };
    selected_windows(snapshot, session_index)
        .iter()
        .position(|window| window.index == current.window_index)
        .unwrap_or(0)
}

pub(super) fn selected_pane_index(
    snapshot: &TmuxManagerSnapshot,
    session_index: usize,
    window_index: usize,
) -> usize {
    let Some(current) = snapshot.current.as_ref() else {
        return 0;
    };
    selected_panes(snapshot, session_index, window_index)
        .iter()
        .position(|pane| pane.id == current.pane_id)
        .unwrap_or(0)
}

pub(super) fn selected_windows(
    snapshot: &TmuxManagerSnapshot,
    session_index: usize,
) -> Vec<&TmuxWindow> {
    let Some(session) = snapshot.state.sessions.get(session_index) else {
        return Vec::new();
    };
    snapshot
        .state
        .windows
        .iter()
        .filter(|window| window.session_name == session.name)
        .collect()
}

pub(super) fn selected_panes(
    snapshot: &TmuxManagerSnapshot,
    session_index: usize,
    window_index: usize,
) -> Vec<&TmuxPane> {
    let Some(window) = selected_windows(snapshot, session_index)
        .get(window_index)
        .copied()
    else {
        return Vec::new();
    };
    snapshot
        .state
        .panes
        .iter()
        .filter(|pane| {
            pane.session_name == window.session_name && pane.window_index == window.index
        })
        .collect()
}

pub(super) fn window_label(window: &TmuxWindow) -> String {
    format!("{}:{}", window.index, window.name)
}
