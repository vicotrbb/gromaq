//! User-facing tmux manager hint rows.

use super::selection::selected_windows;
use super::state::TmuxManagerPanelState;
use crate::tmux::{ActionId, TmuxAction, TmuxManagerSnapshot, TmuxManagerStatus};

pub(super) fn action_hint(action: &TmuxAction) -> String {
    match action.key_binding {
        Some(key) => format!("{} | {key}", action.tmux_command),
        None => action.tmux_command.to_owned(),
    }
}

pub(super) fn action_choice_label(
    action: &TmuxAction,
    selected: bool,
    snapshot: &TmuxManagerSnapshot,
    panel: &TmuxManagerPanelState,
) -> String {
    let mut label = match super::input::action_shortcut(action.id) {
        Some(shortcut) => format!("{shortcut} {}", action.stable_id),
        None => action.stable_id.to_owned(),
    };
    if !action_available(action, snapshot, panel) {
        label.push_str("[needs-active]");
    }
    if selected {
        label.push('*');
    }
    label
}

fn action_available(
    action: &TmuxAction,
    snapshot: &TmuxManagerSnapshot,
    panel: &TmuxManagerPanelState,
) -> bool {
    !action.requires_active_tmux
        || action.can_run_outside_tmux
        || snapshot.current.is_some()
        || action_has_selected_target(action.id, snapshot, panel)
}

fn action_has_selected_target(
    action_id: ActionId,
    snapshot: &TmuxManagerSnapshot,
    panel: &TmuxManagerPanelState,
) -> bool {
    match action_id {
        ActionId::SplitPaneRight
        | ActionId::SplitPaneDown
        | ActionId::SelectPane
        | ActionId::KillPane => panel.selected_pane_id(snapshot).is_some(),
        ActionId::NewWindow | ActionId::RenameSession | ActionId::KillSession => {
            panel.selected_session_name(snapshot).is_some()
        }
        ActionId::RenameWindow | ActionId::KillWindow => {
            selected_windows(snapshot, panel.selected_session)
                .get(panel.selected_window)
                .is_some()
        }
        _ => false,
    }
}

pub(super) fn hint_row(snapshot: &TmuxManagerSnapshot) -> String {
    if snapshot.status == TmuxManagerStatus::Missing {
        return "tmux missing; install tmux or disable tmux workflows | Esc close".to_owned();
    }
    if snapshot.state.sessions.is_empty() {
        return "No tmux server; Enter start-session to create a tmux session | Esc close"
            .to_owned();
    }
    if snapshot.current.is_none() {
        return "Outside tmux; Enter attach-session to attach selected session | Esc close"
            .to_owned();
    }
    format!("Shortcuts {} | Esc close", super::input::shortcut_hint())
}
