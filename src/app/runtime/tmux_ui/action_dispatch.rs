use crate::app::{NativePtySessionIo, TmuxManagerKeyOutcome};
use crate::tmux::{
    ActionId, TmuxAction, TmuxActionResult, TmuxCommandRunner, TmuxError, TmuxTerminalCommand,
    TmuxWorkspaceResult,
};

use super::super::NativeTerminalRuntime;

impl<S> NativeTerminalRuntime<S> {
    /// Dispatch an action-producing tmux manager key outcome through a command runner.
    pub fn dispatch_tmux_manager_action<R>(
        &mut self,
        outcome: TmuxManagerKeyOutcome,
        runner: &R,
    ) -> Option<TmuxActionResult>
    where
        S: NativePtySessionIo,
        R: TmuxCommandRunner,
    {
        let (result, started_session) = {
            let (Some(snapshot), Some(panel)) = (
                self.tmux_manager_snapshot.as_ref(),
                self.tmux_manager_panel.as_mut(),
            ) else {
                return None;
            };
            let pending_name = panel.pending_action_name().map(str::to_owned);
            let result = panel.dispatch_action_outcome(outcome, snapshot, runner);
            let started_session = pending_name.filter(|_| {
                matches!(
                    result,
                    Some(TmuxActionResult::Success {
                        action_id: ActionId::StartSession,
                        ..
                    })
                )
            });
            (result, started_session)
        };
        if let Some(session) = started_session {
            let _ =
                self.send_pty_input(&TmuxTerminalCommand::attach_session(&session).to_pty_input());
        }
        if result.is_some() {
            self.sync_tmux_status_feedback_from_panel();
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
        self.sync_tmux_status_feedback_from_panel();
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
            self.sync_tmux_status_feedback_from_panel();
            self.terminal.invalidate_viewport();
        }
        result
    }
}
