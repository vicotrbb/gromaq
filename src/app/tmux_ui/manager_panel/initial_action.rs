//! Initial tmux manager action selection.

use crate::tmux::{ActionId, TmuxManagerSnapshot};

pub(super) fn initial_selected_action(snapshot: &TmuxManagerSnapshot) -> usize {
    if snapshot.state.sessions.is_empty() {
        return action_index(ActionId::StartSession);
    }
    if snapshot.current.is_none() {
        return action_index(ActionId::AttachSession);
    }
    action_index(ActionId::SplitPaneRight)
}

fn action_index(action_id: ActionId) -> usize {
    super::input::panel_actions()
        .iter()
        .position(|candidate| *candidate == action_id)
        .unwrap_or(0)
}
