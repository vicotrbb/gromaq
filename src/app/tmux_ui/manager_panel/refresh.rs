//! Refresh helpers for preserving manager panel interaction state.

use super::input::panel_actions;
use super::selection::{selected_panes, selected_windows};
use super::state::{TmuxManagerFocus, TmuxManagerPanelState};
use super::workspaces::TmuxWorkspaceUiPreset;
use crate::tmux::TmuxManagerSnapshot;

impl TmuxManagerPanelState {
    /// Build a refreshed panel while preserving the user's current control region.
    pub fn refresh_for_snapshot_with_workspaces(
        &self,
        snapshot: &TmuxManagerSnapshot,
        workspace_presets: Vec<TmuxWorkspaceUiPreset>,
    ) -> Self {
        let mut refreshed = Self::open_for_snapshot_with_workspaces(snapshot, workspace_presets);
        refreshed.focus = refreshed_focus(self.focus, refreshed.workspace_presets.len());
        refreshed.selected_session =
            clamp_index(self.selected_session, snapshot.state.sessions.len());
        refreshed.selected_window = clamp_index(
            self.selected_window,
            selected_windows(snapshot, refreshed.selected_session).len(),
        );
        refreshed.selected_pane = clamp_index(
            self.selected_pane,
            selected_panes(
                snapshot,
                refreshed.selected_session,
                refreshed.selected_window,
            )
            .len(),
        );
        refreshed.selected_workspace =
            clamp_index(self.selected_workspace, refreshed.workspace_presets.len());
        refreshed.selected_action = clamp_index(self.selected_action, panel_actions().len());
        refreshed
    }
}

fn refreshed_focus(focus: TmuxManagerFocus, workspace_count: usize) -> TmuxManagerFocus {
    if focus == TmuxManagerFocus::Workspaces && workspace_count == 0 {
        return TmuxManagerFocus::Actions;
    }
    focus
}

fn clamp_index(index: usize, len: usize) -> usize {
    if len == 0 { 0 } else { index.min(len - 1) }
}
