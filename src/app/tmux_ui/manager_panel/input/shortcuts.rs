//! Discoverable action shortcuts for the native tmux manager panel.

use crate::tmux::ActionId;
use winit::keyboard::Key;

pub(super) fn shortcut_action(key: &Key) -> Option<ActionId> {
    let Key::Character(character) = key else {
        return None;
    };
    if character == "?" {
        Some(ActionId::ShowHelp)
    } else if character.eq_ignore_ascii_case("a") {
        Some(ActionId::AttachSession)
    } else if character.eq_ignore_ascii_case("c") {
        Some(ActionId::NewWindow)
    } else if character.eq_ignore_ascii_case("d") {
        Some(ActionId::DetachSession)
    } else if character.eq_ignore_ascii_case("e") {
        Some(ActionId::RenameWindow)
    } else if character.eq_ignore_ascii_case("s") {
        Some(ActionId::SplitPaneRight)
    } else if character.eq_ignore_ascii_case("t") {
        Some(ActionId::StartSession)
    } else if character.eq_ignore_ascii_case("u") {
        Some(ActionId::RenameSession)
    } else if character.eq_ignore_ascii_case("v") {
        Some(ActionId::SplitPaneDown)
    } else if character.eq_ignore_ascii_case("m") {
        Some(ActionId::SelectPane)
    } else if character.eq_ignore_ascii_case("n") {
        Some(ActionId::NextWindow)
    } else if character.eq_ignore_ascii_case("p") {
        Some(ActionId::PreviousWindow)
    } else if character.eq_ignore_ascii_case("q") {
        Some(ActionId::KillSession)
    } else if character.eq_ignore_ascii_case("w") {
        Some(ActionId::KillWindow)
    } else if character.eq_ignore_ascii_case("x") {
        Some(ActionId::KillPane)
    } else if character.eq_ignore_ascii_case("z") {
        Some(ActionId::ZoomPane)
    } else {
        None
    }
}

pub(super) fn action_shortcut(action_id: ActionId) -> Option<&'static str> {
    match action_id {
        ActionId::AttachSession => Some("a"),
        ActionId::NewWindow => Some("c"),
        ActionId::DetachSession => Some("d"),
        ActionId::RenameWindow => Some("e"),
        ActionId::SplitPaneRight => Some("s"),
        ActionId::StartSession => Some("t"),
        ActionId::RenameSession => Some("u"),
        ActionId::SplitPaneDown => Some("v"),
        ActionId::SelectPane => Some("m"),
        ActionId::NextWindow => Some("n"),
        ActionId::PreviousWindow => Some("p"),
        ActionId::KillSession => Some("q"),
        ActionId::KillWindow => Some("w"),
        ActionId::KillPane => Some("x"),
        ActionId::ZoomPane => Some("z"),
        ActionId::ShowHelp => Some("?"),
    }
}

pub(super) fn is_refresh_shortcut(key: &Key) -> bool {
    let Key::Character(character) = key else {
        return false;
    };
    character.eq_ignore_ascii_case("r")
}

pub(super) fn shortcut_hint() -> &'static str {
    "shortcuts ? help q kill-session a attach-session c new-window d detach-session e rename-window s split-right t start-session u rename-session v split-down m select-pane n next-window p previous-window z zoom-pane r refresh w kill-window x kill-pane"
}
