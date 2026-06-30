//! Native tmux manager panel keyboard handling.

mod navigation;

use super::state::{TmuxManagerFocus, TmuxManagerPanelState};
use crate::tmux::{ActionId, TmuxAction, TmuxManagerSnapshot};
use winit::keyboard::{Key, ModifiersState, NamedKey};

const PANEL_ACTIONS: [ActionId; 2] = [ActionId::SplitPaneRight, ActionId::KillWindow];

/// Result of handling a key while the tmux manager panel may be open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmuxManagerKeyOutcome {
    /// The panel did not use the key.
    Ignored,
    /// The panel consumed the key without requesting an action.
    Consumed,
    /// The panel closed.
    Close,
    /// The panel requested a safe action.
    ActionRequested(ActionId),
    /// The panel requested explicit confirmation for a destructive action.
    ConfirmationRequired(ActionId),
    /// The panel confirmed a previously requested destructive action.
    ConfirmedAction(ActionId),
    /// The panel requested launching the selected workspace preset.
    WorkspaceLaunchRequested,
}

impl TmuxManagerPanelState {
    /// Handle native keyboard input for the manager panel.
    pub fn handle_key(
        &mut self,
        key: &Key,
        modifiers: ModifiersState,
        snapshot: &TmuxManagerSnapshot,
    ) -> TmuxManagerKeyOutcome {
        if !self.open || !modifiers.is_empty() {
            return TmuxManagerKeyOutcome::Ignored;
        }
        if self.confirmation.is_some() {
            return self.handle_confirmation_key(key);
        }
        match key {
            Key::Named(NamedKey::Escape) => {
                self.close();
                TmuxManagerKeyOutcome::Close
            }
            Key::Named(NamedKey::ArrowRight) => {
                self.focus_next();
                TmuxManagerKeyOutcome::Consumed
            }
            Key::Named(NamedKey::ArrowLeft) => {
                self.focus_previous();
                TmuxManagerKeyOutcome::Consumed
            }
            Key::Named(NamedKey::ArrowDown) => {
                self.move_next(snapshot);
                TmuxManagerKeyOutcome::Consumed
            }
            Key::Named(NamedKey::ArrowUp) => {
                self.move_previous(snapshot);
                TmuxManagerKeyOutcome::Consumed
            }
            Key::Named(NamedKey::Enter) => self.activate_selected_action(),
            Key::Character(character) if character.eq_ignore_ascii_case("l") => {
                self.focus_next();
                TmuxManagerKeyOutcome::Consumed
            }
            Key::Character(character) if character.eq_ignore_ascii_case("h") => {
                self.focus_previous();
                TmuxManagerKeyOutcome::Consumed
            }
            Key::Character(character) if character.eq_ignore_ascii_case("j") => {
                self.move_next(snapshot);
                TmuxManagerKeyOutcome::Consumed
            }
            Key::Character(character) if character.eq_ignore_ascii_case("k") => {
                self.move_previous(snapshot);
                TmuxManagerKeyOutcome::Consumed
            }
            _ => TmuxManagerKeyOutcome::Ignored,
        }
    }

    fn handle_confirmation_key(&mut self, key: &Key) -> TmuxManagerKeyOutcome {
        match key {
            Key::Named(NamedKey::Escape) => {
                self.cancel_confirmation();
                TmuxManagerKeyOutcome::Consumed
            }
            Key::Character(character) if character.eq_ignore_ascii_case("y") => {
                let action_id = self.confirmation_action;
                self.cancel_confirmation();
                match action_id {
                    Some(action_id) => TmuxManagerKeyOutcome::ConfirmedAction(action_id),
                    None => TmuxManagerKeyOutcome::Consumed,
                }
            }
            Key::Character(character) if character.eq_ignore_ascii_case("n") => {
                self.cancel_confirmation();
                TmuxManagerKeyOutcome::Consumed
            }
            _ => TmuxManagerKeyOutcome::Consumed,
        }
    }

    fn activate_selected_action(&mut self) -> TmuxManagerKeyOutcome {
        if self.focus == TmuxManagerFocus::Workspaces {
            return if self.workspace_presets.is_empty() {
                TmuxManagerKeyOutcome::Consumed
            } else {
                TmuxManagerKeyOutcome::WorkspaceLaunchRequested
            };
        }
        let action_id = PANEL_ACTIONS
            .get(self.selected_action)
            .copied()
            .unwrap_or(ActionId::SplitPaneRight);
        let action = TmuxAction::by_id(action_id).expect("panel action is registered");
        if action.confirmation_required {
            self.request_action(action.stable_id, true);
            return TmuxManagerKeyOutcome::ConfirmationRequired(action_id);
        }
        self.request_action(action.stable_id, false);
        TmuxManagerKeyOutcome::ActionRequested(action_id)
    }
}

pub(super) fn panel_actions() -> &'static [ActionId] {
    &PANEL_ACTIONS
}
