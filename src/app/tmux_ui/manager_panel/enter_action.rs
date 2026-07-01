//! Focus-sensitive default action selection for native tmux manager Enter.

use super::input::panel_actions;
use super::state::{TmuxManagerFocus, TmuxManagerPanelState};
use crate::tmux::{ActionId, TmuxManagerSnapshot};

pub(super) fn enter_action_id(
    snapshot: &TmuxManagerSnapshot,
    panel: &TmuxManagerPanelState,
) -> ActionId {
    match panel.focus {
        TmuxManagerFocus::Sessions if panel.selected_session_name(snapshot).is_some() => {
            ActionId::AttachSession
        }
        TmuxManagerFocus::Sessions => ActionId::StartSession,
        TmuxManagerFocus::Panes => ActionId::SelectPane,
        _ => panel_actions()
            .get(panel.selected_action)
            .copied()
            .unwrap_or(ActionId::SplitPaneRight),
    }
}
