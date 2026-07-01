//! Row label rendering for the native tmux manager panel.

use super::selection::{selected_panes, selected_windows, window_label};
use super::state::TmuxManagerPanelState;
use super::target::{
    is_current_pane, is_current_session, is_current_window, pane_command_label, pane_dimensions,
};
use crate::tmux::{TmuxManagerSnapshot, TmuxPane};

pub(super) fn session_row(snapshot: &TmuxManagerSnapshot, panel: &TmuxManagerPanelState) -> String {
    if snapshot.state.sessions.is_empty() {
        return "none".to_owned();
    }
    snapshot
        .state
        .sessions
        .iter()
        .enumerate()
        .map(|(index, session)| {
            selected_label(
                &session.name,
                index == panel.selected_session,
                is_current_session(snapshot, &session.name),
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn window_row(snapshot: &TmuxManagerSnapshot, panel: &TmuxManagerPanelState) -> String {
    let windows = selected_windows(snapshot, panel.selected_session);
    if windows.is_empty() {
        return "none".to_owned();
    }
    windows
        .iter()
        .enumerate()
        .map(|(index, window)| {
            selected_label(
                &window_label(window),
                index == panel.selected_window,
                is_current_window(snapshot, &window.session_name, window.index),
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn pane_row(snapshot: &TmuxManagerSnapshot, panel: &TmuxManagerPanelState) -> String {
    let panes = selected_panes(snapshot, panel.selected_session, panel.selected_window);
    if panes.is_empty() {
        return "none".to_owned();
    }
    panes
        .iter()
        .enumerate()
        .map(|(index, pane)| {
            pane_label(
                pane,
                index == panel.selected_pane,
                is_current_pane(snapshot, pane),
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn pane_label(pane: &TmuxPane, selected: bool, current: bool) -> String {
    let mut command = pane_command_label(pane);
    if selected {
        command.push('*');
    }
    let dimensions = pane_dimensions(pane);
    let current_marker = if current { "@" } else { "" };
    format!("{} {}{}{}", pane.id, command, dimensions, current_marker)
}

fn selected_label(label: &str, selected: bool, current: bool) -> String {
    format!(
        "{label}{}{}",
        if selected { "*" } else { "" },
        if current && !selected { "@" } else { "" }
    )
}
