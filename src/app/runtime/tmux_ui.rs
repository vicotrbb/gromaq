use crate::tmux::{
    ActionId, TmuxAction, TmuxActionResult, TmuxCommandRunner, TmuxError, TmuxManagerSnapshot,
    TmuxTerminalCommand, TmuxWorkspaceResult,
};

use super::NativeTerminalRuntime;
use crate::app::{
    NativePtySessionIo, TmuxManagerKeyOutcome, TmuxManagerPanelState, TmuxUiSnapshot,
    TmuxWorkspaceUiPreset,
};

impl<S> NativeTerminalRuntime<S> {
    /// Retain a tmux status snapshot for the normal native render path.
    pub fn set_tmux_status_snapshot(&mut self, snapshot: TmuxUiSnapshot) {
        self.tmux_status_snapshot = Some(snapshot);
        self.terminal.invalidate_viewport();
    }

    /// Clear the retained tmux status snapshot from the normal native render path.
    pub fn clear_tmux_status_snapshot(&mut self) {
        self.tmux_status_snapshot = None;
        self.terminal.invalidate_viewport();
    }

    /// Return whether the last rendered frame applied the native tmux status strip.
    pub fn last_rendered_tmux_status_strip(&self) -> bool {
        self.last_rendered_tmux_status_strip
    }

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
            self.tmux_status_snapshot = Some(TmuxUiSnapshot::from_manager_snapshot(&snapshot));
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

    /// Refresh the open tmux manager panel with a newly read snapshot.
    pub fn refresh_tmux_manager_panel(&mut self, snapshot: TmuxManagerSnapshot) {
        let Some(panel) = self.tmux_manager_panel.as_ref() else {
            return;
        };
        if !panel.is_open() {
            return;
        }
        let workspace_presets = panel.workspace_presets().to_vec();
        self.tmux_manager_panel = Some(TmuxManagerPanelState::open_for_snapshot_with_workspaces(
            &snapshot,
            workspace_presets,
        ));
        self.tmux_manager_snapshot = Some(snapshot);
        if let Some(snapshot) = self.tmux_manager_snapshot.as_ref() {
            self.tmux_status_snapshot = Some(TmuxUiSnapshot::from_manager_snapshot(snapshot));
        }
        self.terminal.invalidate_viewport();
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

    /// Dispatch a manager action that must be entered through the retained terminal PTY.
    pub fn dispatch_tmux_manager_terminal_action(
        &mut self,
        outcome: TmuxManagerKeyOutcome,
    ) -> Option<TmuxActionResult>
    where
        S: NativePtySessionIo,
    {
        if !matches!(
            outcome,
            TmuxManagerKeyOutcome::ActionRequested(ActionId::AttachSession)
        ) {
            return None;
        }
        let session = {
            let snapshot = self.tmux_manager_snapshot.as_ref()?;
            let panel = self.tmux_manager_panel.as_ref()?;
            panel.selected_session_name(snapshot)?.to_owned()
        };
        let action = TmuxAction::by_id(ActionId::AttachSession).expect("attach action exists");
        let result = match self
            .send_pty_input(&TmuxTerminalCommand::attach_session(&session).to_pty_input())
        {
            Ok(()) => TmuxActionResult::Success {
                action_id: ActionId::AttachSession,
                teaching_hint: format!("tmux command: {}", action.tmux_command),
            },
            Err(error) => TmuxActionResult::Skipped {
                action_id: ActionId::AttachSession,
                reason: error.to_string(),
                teaching_hint: format!("tmux command: {}", action.tmux_command),
            },
        };
        if let Some(panel) = self.tmux_manager_panel.as_mut() {
            panel.record_action_feedback(match &result {
                TmuxActionResult::Success { .. } => "attach-session success".to_owned(),
                TmuxActionResult::Skipped { reason, .. } => {
                    format!("attach-session skipped: {reason}")
                }
                _ => "attach-session skipped".to_owned(),
            });
        }
        self.terminal.invalidate_viewport();
        Some(result)
    }

    /// Dispatch a workspace-launch tmux manager key outcome through the workspace launcher.
    pub fn dispatch_tmux_manager_workspace<R>(
        &mut self,
        outcome: TmuxManagerKeyOutcome,
        runner: &R,
    ) -> Option<Result<TmuxWorkspaceResult, TmuxError>>
    where
        S: NativePtySessionIo,
        R: TmuxCommandRunner,
    {
        if !matches!(outcome, TmuxManagerKeyOutcome::WorkspaceLaunchRequested) {
            return None;
        }
        let panel = self.tmux_manager_panel.as_mut()?;
        let result = panel.ensure_selected_workspace_started(runner);
        if let Some(Ok(workspace)) = result.as_ref() {
            let session = match workspace {
                TmuxWorkspaceResult::Existing { session }
                | TmuxWorkspaceResult::Attached { session }
                | TmuxWorkspaceResult::Started { session, .. } => session,
            };
            let _ =
                self.send_pty_input(&TmuxTerminalCommand::attach_session(session).to_pty_input());
        }
        if result.is_some() {
            self.terminal.invalidate_viewport();
        }
        result
    }
}
