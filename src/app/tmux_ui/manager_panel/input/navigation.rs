//! Focus and selection navigation for the native tmux manager panel.

use super::super::selection::{selected_panes, selected_windows};
use super::super::state::{TmuxManagerFocus, TmuxManagerPanelState};
use super::PANEL_ACTIONS;
use crate::tmux::TmuxManagerSnapshot;

impl TmuxManagerPanelState {
    pub(super) fn focus_previous(&mut self) {
        self.focus = match self.focus {
            TmuxManagerFocus::Sessions => TmuxManagerFocus::Actions,
            TmuxManagerFocus::Windows => TmuxManagerFocus::Sessions,
            TmuxManagerFocus::Panes => TmuxManagerFocus::Windows,
            TmuxManagerFocus::Workspaces => TmuxManagerFocus::Panes,
            TmuxManagerFocus::Actions if self.workspace_presets.is_empty() => {
                TmuxManagerFocus::Panes
            }
            TmuxManagerFocus::Actions => TmuxManagerFocus::Workspaces,
        };
    }

    pub(super) fn move_next(&mut self, snapshot: &TmuxManagerSnapshot) {
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
            TmuxManagerFocus::Workspaces => {
                self.selected_workspace =
                    next_index(self.selected_workspace, self.workspace_presets.len());
            }
            TmuxManagerFocus::Actions => {
                self.selected_action = next_index(self.selected_action, PANEL_ACTIONS.len());
            }
        }
    }

    pub(super) fn move_previous(&mut self, snapshot: &TmuxManagerSnapshot) {
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
            TmuxManagerFocus::Workspaces => {
                self.selected_workspace =
                    previous_index(self.selected_workspace, self.workspace_presets.len());
            }
            TmuxManagerFocus::Actions => {
                self.selected_action = previous_index(self.selected_action, PANEL_ACTIONS.len());
            }
        }
    }
}

fn next_index(current: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        (current + 1).min(len - 1)
    }
}

fn previous_index(current: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        current.saturating_sub(1)
    }
}
