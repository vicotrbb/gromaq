//! Native tmux manager panel keyboard handling.

mod activation;
mod navigation;
mod shortcuts;

use super::state::TmuxManagerPanelState;
use crate::tmux::{ActionId, TmuxAction, TmuxManagerSnapshot};
use shortcuts::shortcut_action;
use winit::keyboard::{Key, ModifiersState, NamedKey};

const PANEL_ACTIONS: [ActionId; 16] = [
    ActionId::SplitPaneRight,
    ActionId::KillWindow,
    ActionId::AttachSession,
    ActionId::StartSession,
    ActionId::DetachSession,
    ActionId::SplitPaneDown,
    ActionId::NewWindow,
    ActionId::RenameSession,
    ActionId::RenameWindow,
    ActionId::NextWindow,
    ActionId::PreviousWindow,
    ActionId::ZoomPane,
    ActionId::SelectPane,
    ActionId::KillPane,
    ActionId::KillSession,
    ActionId::ShowHelp,
];

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
        if self.action_input.is_some() {
            return self.handle_action_input_key(key);
        }
        if self.confirmation.is_some() {
            return self.handle_confirmation_key(key);
        }
        if let Some(action_id) = shortcut_action(key) {
            return self.activate_action(action_id);
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

    fn handle_action_input_key(&mut self, key: &Key) -> TmuxManagerKeyOutcome {
        let Some(input) = self.action_input.as_mut() else {
            return TmuxManagerKeyOutcome::Consumed;
        };
        match key {
            Key::Named(NamedKey::Escape) => {
                self.action_input = None;
                self.pending_action_name = None;
                TmuxManagerKeyOutcome::Consumed
            }
            Key::Named(NamedKey::Backspace) => {
                input.value.pop();
                TmuxManagerKeyOutcome::Consumed
            }
            Key::Named(NamedKey::Enter) => {
                let value = input.value.trim().to_owned();
                let action_id = input.action_id;
                if value.is_empty() {
                    return TmuxManagerKeyOutcome::Consumed;
                }
                let action = TmuxAction::by_id(action_id).expect("input action is registered");
                self.action_input = None;
                self.pending_action_name = Some(value);
                self.pending_action = Some(action.stable_id.to_owned());
                self.confirmation = None;
                self.confirmation_action = None;
                TmuxManagerKeyOutcome::ActionRequested(action_id)
            }
            Key::Character(character)
                if input.value.len() < 64 && !character.chars().any(char::is_control) =>
            {
                input.value.push_str(character);
                TmuxManagerKeyOutcome::Consumed
            }
            _ => TmuxManagerKeyOutcome::Consumed,
        }
    }
}

pub(super) fn panel_actions() -> &'static [ActionId] {
    &PANEL_ACTIONS
}

pub(super) fn shortcut_hint() -> &'static str {
    shortcuts::shortcut_hint()
}
