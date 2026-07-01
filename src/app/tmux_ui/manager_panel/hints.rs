//! User-facing tmux manager hint rows.

use crate::tmux::{TmuxAction, TmuxManagerSnapshot, TmuxManagerStatus};

pub(super) fn action_hint(action: &TmuxAction) -> String {
    match action.key_binding {
        Some(key) => format!("{} | {key}", action.tmux_command),
        None => action.tmux_command.to_owned(),
    }
}

pub(super) fn action_choice_label(action: &TmuxAction, selected: bool) -> String {
    let mut label = match super::input::action_shortcut(action.id) {
        Some(shortcut) => format!("{shortcut} {}", action.stable_id),
        None => action.stable_id.to_owned(),
    };
    if selected {
        label.push('*');
    }
    label
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
