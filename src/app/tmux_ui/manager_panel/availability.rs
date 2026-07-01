//! Manager action availability helpers shared by rendering and input.

use super::selection::selected_windows;
use super::state::TmuxManagerPanelState;
use crate::tmux::{ActionId, TmuxAction, TmuxManagerSnapshot};

pub(super) fn action_available(
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
