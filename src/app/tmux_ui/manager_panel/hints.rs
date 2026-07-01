//! User-facing tmux manager hint rows.

use super::availability::action_available;
use super::state::TmuxManagerPanelState;
use crate::tmux::{TmuxAction, TmuxManagerSnapshot, TmuxManagerStatus, shell_quote};

pub(super) fn action_hint(action: &TmuxAction) -> String {
    match action.key_binding {
        Some(key) => format!("{} | {key}", action.tmux_command),
        None => action.tmux_command.to_owned(),
    }
}

pub(super) fn help_catalog() -> String {
    let mut entries = vec!["r refresh tmux refresh snapshot no tmux key".to_owned()];
    entries.extend(TmuxAction::registry().iter().filter_map(|action| {
        super::input::action_shortcut(action.id).map(|shortcut| {
            format!(
                "{shortcut} {} {} {}",
                action.stable_id,
                action.tmux_command,
                action.key_binding.unwrap_or("no tmux key")
            )
        })
    }));
    format!("tmux help | {} | Esc close", entries.join(" | "))
}

pub(super) fn action_choice_label(
    action: &TmuxAction,
    selected: bool,
    snapshot: &TmuxManagerSnapshot,
    panel: &TmuxManagerPanelState,
) -> String {
    let mut label = match super::input::action_shortcut(action.id) {
        Some(shortcut) => format!(
            "{shortcut} {} {}",
            action.stable_id,
            action_command_teaching(action)
        ),
        None => format!("{} {}", action.stable_id, action_command_teaching(action)),
    };
    if !action_available(action, snapshot, panel) {
        label.push_str("[needs-active]");
    }
    if selected {
        label.push('*');
    }
    label
}

fn action_command_teaching(action: &TmuxAction) -> String {
    match action.key_binding {
        Some(key) => format!("{} {key}", action.tmux_command),
        None => action.tmux_command.to_owned(),
    }
}

pub(super) fn enter_action_label(
    action: &TmuxAction,
    snapshot: &TmuxManagerSnapshot,
    panel: &TmuxManagerPanelState,
) -> String {
    let mut label = action.stable_id.to_owned();
    if !action_available(action, snapshot, panel) {
        label.push_str("[needs-active]");
    }
    label
}

pub(super) fn hint_row(snapshot: &TmuxManagerSnapshot) -> String {
    if snapshot.status == TmuxManagerStatus::Missing {
        return "tmux missing; install tmux or disable tmux workflows | r refresh | ? help | Esc close"
            .to_owned();
    }
    if snapshot.state.sessions.is_empty() {
        return "No tmux server; Enter start-session to create | r refresh | ? help | Esc close"
            .to_owned();
    }
    if snapshot.current.is_none() {
        let session = snapshot
            .state
            .sessions
            .first()
            .map(|session| format!(" {}", shell_quote(&session.name)))
            .unwrap_or_default();
        return format!(
            "Outside tmux; Enter attach-session{session} | r refresh | ? help | Esc close"
        );
    }
    format!(
        "Shortcuts ? help | r refresh | Esc close | {}",
        super::input::shortcut_hint()
    )
}
