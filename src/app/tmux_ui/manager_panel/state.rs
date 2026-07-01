//! Native tmux manager panel state.

use super::initial_action::initial_selected_action;
use super::selection::{
    selected_pane_index, selected_panes, selected_session_index, selected_window_index,
    selected_windows, window_label,
};
use super::workspaces::TmuxWorkspaceUiPreset;
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
    /// Workspace preset row focus.
    Workspaces,
    /// Action row focus.
    Actions,
}

/// Inline text input for tmux actions that need a user-supplied name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxActionInputState {
    pub(super) action_id: ActionId,
    pub(super) value: String,
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
    pub(super) pending_action_name: Option<String>,
    pub(super) confirmation: Option<String>,
    pub(super) confirmation_action: Option<ActionId>,
    pub(super) action_input: Option<TmuxActionInputState>,
    pub(super) last_action_feedback: Option<String>,
    pub(super) workspace_presets: Vec<TmuxWorkspaceUiPreset>,
    pub(super) selected_workspace: usize,
    pub(super) workspace_feedback: Option<String>,
}

impl TmuxManagerPanelState {
    /// Build an open panel state from the current tmux manager snapshot.
    pub fn open_for_snapshot(snapshot: &TmuxManagerSnapshot) -> Self {
        Self::open_for_snapshot_with_workspaces(snapshot, Vec::new())
    }

    /// Build an open panel state with visible workspace presets.
    pub fn open_for_snapshot_with_workspaces(
        snapshot: &TmuxManagerSnapshot,
        workspace_presets: Vec<TmuxWorkspaceUiPreset>,
    ) -> Self {
        let selected_session = selected_session_index(snapshot);
        let selected_window = selected_window_index(snapshot, selected_session);
        let selected_pane = selected_pane_index(snapshot, selected_session, selected_window);
        Self {
            open: true,
            focus: TmuxManagerFocus::Sessions,
            selected_session,
            selected_window,
            selected_pane,
            selected_action: initial_selected_action(snapshot),
            pending_action: None,
            pending_action_name: None,
            confirmation: None,
            confirmation_action: None,
            action_input: None,
            last_action_feedback: None,
            workspace_presets,
            selected_workspace: 0,
            workspace_feedback: None,
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
        self.action_input = None;
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
            TmuxManagerFocus::Panes if self.workspace_presets.is_empty() => {
                TmuxManagerFocus::Actions
            }
            TmuxManagerFocus::Panes => TmuxManagerFocus::Workspaces,
            TmuxManagerFocus::Workspaces => TmuxManagerFocus::Actions,
            TmuxManagerFocus::Actions => TmuxManagerFocus::Sessions,
        };
    }

    /// Request an action, asking for confirmation when it is destructive.
    pub fn request_action(&mut self, stable_id: &str, destructive: bool) {
        self.pending_action_name = None;
        if destructive {
            let action = TmuxAction::by_stable_id(stable_id);
            let command = action
                .map(|action| action.tmux_command)
                .unwrap_or("<unknown tmux command>");
            self.confirmation = Some(format!(
                "confirm {stable_id} ({command}) with y, n/Esc cancels"
            ));
            self.confirmation_action = action.map(|action| action.id);
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

    /// Return the pending user-supplied action name.
    pub fn pending_action_name(&self) -> Option<&str> {
        self.pending_action_name.as_deref()
    }

    /// Return the active action input prompt text.
    pub fn action_input_prompt(&self) -> Option<String> {
        let input = self.action_input.as_ref()?;
        let action = TmuxAction::by_id(input.action_id)?;
        Some(format!("{} name: {}", action.stable_id, input.value))
    }

    /// Return the latest action execution feedback.
    pub fn last_action_feedback(&self) -> Option<&str> {
        self.last_action_feedback.as_deref()
    }

    /// Return configured workspace presets visible in this panel.
    pub fn workspace_presets(&self) -> &[TmuxWorkspaceUiPreset] {
        &self.workspace_presets
    }

    /// Record action execution feedback from a runtime-specific action path.
    pub fn record_action_feedback(&mut self, feedback: impl Into<String>) {
        self.last_action_feedback = Some(feedback.into());
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
