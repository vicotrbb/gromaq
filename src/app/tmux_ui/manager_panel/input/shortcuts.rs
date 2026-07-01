//! Discoverable action shortcuts for the native tmux manager panel.

use crate::tmux::ActionId;
use winit::keyboard::Key;

pub(super) fn shortcut_action(key: &Key) -> Option<ActionId> {
    let Key::Character(character) = key else {
        return None;
    };
    if character.eq_ignore_ascii_case("c") {
        Some(ActionId::NewWindow)
    } else if character.eq_ignore_ascii_case("s") {
        Some(ActionId::SplitPaneRight)
    } else if character.eq_ignore_ascii_case("v") {
        Some(ActionId::SplitPaneDown)
    } else if character.eq_ignore_ascii_case("n") {
        Some(ActionId::NextWindow)
    } else if character.eq_ignore_ascii_case("p") {
        Some(ActionId::PreviousWindow)
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

pub(super) fn shortcut_hint() -> &'static str {
    "shortcuts c new-window s split-right v split-down n next-window p previous-window z zoom-pane w kill-window x kill-pane"
}
