//! Native tmux manager panel keyboard handling.

use super::selection::{selected_panes, selected_windows};
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

    fn focus_previous(&mut self) {
        self.focus = match self.focus {
            TmuxManagerFocus::Sessions => TmuxManagerFocus::Actions,
            TmuxManagerFocus::Windows => TmuxManagerFocus::Sessions,
            TmuxManagerFocus::Panes => TmuxManagerFocus::Windows,
            TmuxManagerFocus::Actions => TmuxManagerFocus::Panes,
        };
    }

    fn move_next(&mut self, snapshot: &TmuxManagerSnapshot) {
        match self.focus {
            TmuxManagerFocus::Sessions => {
                self.selected_session =
                    next_index(self.selected_session, snapshot.state.sessions.len());
                self.selected_window = 0;
                self.selected_pane = 0;
            }
            TmuxManagerFocus::Windows => {
                self.selected_window = next_index(
                    self.selected_window,
                    selected_windows(snapshot, self.selected_session).len(),
                );
                self.selected_pane = 0;
            }
            TmuxManagerFocus::Panes => {
                self.selected_pane = next_index(
                    self.selected_pane,
                    selected_panes(snapshot, self.selected_session, self.selected_window).len(),
                );
            }
            TmuxManagerFocus::Actions => {
                self.selected_action = next_index(self.selected_action, PANEL_ACTIONS.len());
            }
        }
    }

    fn move_previous(&mut self, snapshot: &TmuxManagerSnapshot) {
        match self.focus {
            TmuxManagerFocus::Sessions => {
                self.selected_session =
                    previous_index(self.selected_session, snapshot.state.sessions.len());
                self.selected_window = 0;
                self.selected_pane = 0;
            }
            TmuxManagerFocus::Windows => {
                self.selected_window = previous_index(
                    self.selected_window,
                    selected_windows(snapshot, self.selected_session).len(),
                );
                self.selected_pane = 0;
            }
            TmuxManagerFocus::Panes => {
                self.selected_pane = previous_index(
                    self.selected_pane,
                    selected_panes(snapshot, self.selected_session, self.selected_window).len(),
                );
            }
            TmuxManagerFocus::Actions => {
                self.selected_action = previous_index(self.selected_action, PANEL_ACTIONS.len());
            }
        }
    }

    fn activate_selected_action(&mut self) -> TmuxManagerKeyOutcome {
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

fn next_index(current: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (current + 1).min(len - 1)
}

fn previous_index(current: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    current.saturating_sub(1)
}
