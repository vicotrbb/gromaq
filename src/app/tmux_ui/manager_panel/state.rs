//! Native tmux manager panel state.

use super::selection::{
    selected_pane_index, selected_panes, selected_session_index, selected_window_index,
    selected_windows, window_label,
};
use crate::tmux::{ActionId, TmuxAction, TmuxManagerSnapshot};

/// Focus region within the native tmux manager panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TmuxManagerFocus {
    /// Session list focus.
    Sessions,
    /// Window list focus.
    Windows,
    /// Pane list focus.
    Panes,
    /// Action row focus.
    Actions,
}

/// Native tmux manager panel state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxManagerPanelState {
    pub(super) open: bool,
    pub(super) focus: TmuxManagerFocus,
    pub(super) selected_session: usize,
    pub(super) selected_window: usize,
    pub(super) selected_pane: usize,
    pub(super) selected_action: usize,
    pub(super) pending_action: Option<String>,
    pub(super) confirmation: Option<String>,
    pub(super) confirmation_action: Option<ActionId>,
}

impl TmuxManagerPanelState {
    /// Build an open panel state from the current tmux manager snapshot.
    pub fn open_for_snapshot(snapshot: &TmuxManagerSnapshot) -> Self {
        let selected_session = selected_session_index(snapshot);
        let selected_window = selected_window_index(snapshot, selected_session);
        let selected_pane = selected_pane_index(snapshot, selected_session, selected_window);
        Self {
            open: true,
            focus: TmuxManagerFocus::Sessions,
            selected_session,
            selected_window,
            selected_pane,
            selected_action: 0,
            pending_action: None,
            confirmation: None,
            confirmation_action: None,
        }
    }

    /// Return whether the panel is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Close the panel.
    pub fn close(&mut self) {
        self.open = false;
        self.confirmation = None;
        self.confirmation_action = None;
    }

    /// Return the current focus region.
    pub fn focus(&self) -> TmuxManagerFocus {
        self.focus
    }

    /// Move focus to the next panel region.
    pub fn focus_next(&mut self) {
        self.focus = match self.focus {
            TmuxManagerFocus::Sessions => TmuxManagerFocus::Windows,
            TmuxManagerFocus::Windows => TmuxManagerFocus::Panes,
            TmuxManagerFocus::Panes => TmuxManagerFocus::Actions,
            TmuxManagerFocus::Actions => TmuxManagerFocus::Sessions,
        };
    }

    /// Request an action, asking for confirmation when it is destructive.
    pub fn request_action(&mut self, stable_id: &str, destructive: bool) {
        if destructive {
            self.confirmation = Some(format!("confirm {stable_id} with y"));
            self.confirmation_action = action_id_for_stable_id(stable_id);
            self.pending_action = None;
        } else {
            self.pending_action = Some(stable_id.to_owned());
            self.confirmation = None;
            self.confirmation_action = None;
        }
    }

    /// Cancel the active destructive-action confirmation prompt.
    pub fn cancel_confirmation(&mut self) {
        self.confirmation = None;
        self.confirmation_action = None;
    }

    /// Return the current confirmation message.
    pub fn confirmation_message(&self) -> Option<&str> {
        self.confirmation.as_deref()
    }

    /// Return the pending non-destructive action id.
    pub fn pending_action(&self) -> Option<&str> {
        self.pending_action.as_deref()
    }

    /// Return the selected session name for a snapshot.
    pub fn selected_session_name<'a>(&self, snapshot: &'a TmuxManagerSnapshot) -> Option<&'a str> {
        snapshot
            .state
            .sessions
            .get(self.selected_session)
            .map(|session| session.name.as_str())
    }

    /// Return the selected window label for a snapshot.
    pub fn selected_window_label(&self, snapshot: &TmuxManagerSnapshot) -> Option<String> {
        selected_windows(snapshot, self.selected_session)
            .get(self.selected_window)
            .map(|window| window_label(window))
    }

    /// Return the selected pane id for a snapshot.
    pub fn selected_pane_id<'a>(&self, snapshot: &'a TmuxManagerSnapshot) -> Option<&'a str> {
        selected_panes(snapshot, self.selected_session, self.selected_window)
            .get(self.selected_pane)
            .map(|pane| pane.id.as_str())
    }
}

fn action_id_for_stable_id(stable_id: &str) -> Option<ActionId> {
    TmuxAction::by_stable_id(stable_id).map(|action| action.id)
}
