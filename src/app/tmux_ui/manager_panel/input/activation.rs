//! Action activation helpers for native tmux manager keyboard input.

use super::super::state::{TmuxActionInputState, TmuxManagerFocus, TmuxManagerPanelState};
use super::PANEL_ACTIONS;
use crate::app::TmuxManagerKeyOutcome;
use crate::tmux::{ActionId, TmuxAction};

impl TmuxManagerPanelState {
    pub(super) fn activate_selected_action(&mut self) -> TmuxManagerKeyOutcome {
        if self.focus == TmuxManagerFocus::Workspaces {
            return if self.workspace_presets.is_empty() {
                TmuxManagerKeyOutcome::Consumed
            } else {
                TmuxManagerKeyOutcome::WorkspaceLaunchRequested
            };
        }
        if self.focus == TmuxManagerFocus::Panes {
            return self.activate_action(ActionId::SelectPane);
        }
        let action_id = PANEL_ACTIONS
            .get(self.selected_action)
            .copied()
            .unwrap_or(ActionId::SplitPaneRight);
        self.activate_action(action_id)
    }

    pub(super) fn activate_action(&mut self, action_id: ActionId) -> TmuxManagerKeyOutcome {
        let action = TmuxAction::by_id(action_id).expect("panel action is registered");
        if action_needs_name(action_id) {
            self.action_input = Some(TmuxActionInputState {
                action_id,
                value: String::new(),
            });
            self.pending_action = None;
            self.pending_action_name = None;
            self.confirmation = None;
            self.confirmation_action = None;
            return TmuxManagerKeyOutcome::Consumed;
        }
        self.pending_action_name = None;
        if action.confirmation_required {
            self.request_action(action.stable_id, true);
            return TmuxManagerKeyOutcome::ConfirmationRequired(action_id);
        }
        self.request_action(action.stable_id, false);
        TmuxManagerKeyOutcome::ActionRequested(action_id)
    }
}

fn action_needs_name(action_id: ActionId) -> bool {
    matches!(
        action_id,
        ActionId::StartSession | ActionId::RenameSession | ActionId::RenameWindow
    )
}
