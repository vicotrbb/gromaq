use crate::tmux::{
    TmuxActionResult, TmuxCommandRunner, TmuxError, TmuxManagerSnapshot, TmuxWorkspaceResult,
};

use super::NativeTerminalRuntime;
use crate::app::{TmuxManagerKeyOutcome, TmuxManagerPanelState, TmuxWorkspaceUiPreset};

impl<S> NativeTerminalRuntime<S> {
    /// Toggle the native tmux manager panel using a freshly read snapshot.
    pub fn toggle_tmux_manager_panel(&mut self, snapshot: TmuxManagerSnapshot) {
        self.toggle_tmux_manager_panel_with_workspaces(snapshot, Vec::new());
    }

    /// Toggle the native tmux manager panel with configured workspace presets.
    pub fn toggle_tmux_manager_panel_with_workspaces(
        &mut self,
        snapshot: TmuxManagerSnapshot,
        workspace_presets: Vec<TmuxWorkspaceUiPreset>,
    ) {
        if self.tmux_manager_panel_is_open() {
            self.tmux_manager_panel = None;
            self.tmux_manager_snapshot = None;
        } else {
            self.tmux_manager_panel =
                Some(TmuxManagerPanelState::open_for_snapshot_with_workspaces(
                    &snapshot,
                    workspace_presets,
                ));
            self.tmux_manager_snapshot = Some(snapshot);
        }
        self.terminal.invalidate_viewport();
    }

    /// Return whether the native tmux manager panel is open.
    pub fn tmux_manager_panel_is_open(&self) -> bool {
        self.tmux_manager_panel
            .as_ref()
            .is_some_and(TmuxManagerPanelState::is_open)
    }

    /// Let the open tmux manager panel handle a native key before shell input.
    pub fn handle_tmux_manager_key(
        &mut self,
        key: &winit::keyboard::Key,
        modifiers: winit::keyboard::ModifiersState,
    ) -> TmuxManagerKeyOutcome {
        let (Some(snapshot), Some(panel)) = (
            self.tmux_manager_snapshot.as_ref(),
            self.tmux_manager_panel.as_mut(),
        ) else {
            return TmuxManagerKeyOutcome::Ignored;
        };
        let outcome = panel.handle_key(key, modifiers, snapshot);
        if !matches!(outcome, TmuxManagerKeyOutcome::Ignored) {
            self.terminal.invalidate_viewport();
        }
        if !panel.is_open() {
            self.tmux_manager_panel = None;
            self.tmux_manager_snapshot = None;
        }
        outcome
    }

    /// Dispatch an action-producing tmux manager key outcome through a command runner.
    pub fn dispatch_tmux_manager_action<R>(
        &mut self,
        outcome: TmuxManagerKeyOutcome,
        runner: &R,
    ) -> Option<TmuxActionResult>
    where
        R: TmuxCommandRunner,
    {
        let (Some(snapshot), Some(panel)) = (
            self.tmux_manager_snapshot.as_ref(),
            self.tmux_manager_panel.as_mut(),
        ) else {
            return None;
        };
        let result = panel.dispatch_action_outcome(outcome, snapshot, runner);
        if result.is_some() {
            self.terminal.invalidate_viewport();
        }
        result
    }

    /// Dispatch a workspace-launch tmux manager key outcome through the workspace launcher.
    pub fn dispatch_tmux_manager_workspace<R>(
        &mut self,
        outcome: TmuxManagerKeyOutcome,
        runner: &R,
    ) -> Option<Result<TmuxWorkspaceResult, TmuxError>>
    where
        R: TmuxCommandRunner,
    {
        if !matches!(outcome, TmuxManagerKeyOutcome::WorkspaceLaunchRequested) {
            return None;
        }
        let panel = self.tmux_manager_panel.as_mut()?;
        let result = panel.launch_selected_workspace(runner);
        if result.is_some() {
            self.terminal.invalidate_viewport();
        }
        result
    }
}
