//! Native tmux manager action dispatch.

use super::input::TmuxManagerKeyOutcome;
use super::selection::selected_windows;
use super::state::TmuxManagerPanelState;
use crate::tmux::{
    ActionId, TmuxAction, TmuxActionRequest, TmuxActionResult, TmuxActionRunner, TmuxCommandRunner,
    TmuxManagerSnapshot,
};

impl TmuxManagerPanelState {
    /// Dispatch an action-producing key outcome through the tmux action runner.
    pub fn dispatch_action_outcome<R>(
        &mut self,
        outcome: TmuxManagerKeyOutcome,
        snapshot: &TmuxManagerSnapshot,
        runner: &R,
    ) -> Option<TmuxActionResult>
    where
        R: TmuxCommandRunner,
    {
        let (action_id, confirmed) = match outcome {
            TmuxManagerKeyOutcome::ActionRequested(action_id) => (action_id, false),
            TmuxManagerKeyOutcome::ConfirmedAction(action_id) => (action_id, true),
            _ => return None,
        };
        let request = action_request(action_id, confirmed, self, snapshot);
        let result = TmuxActionRunner::new(runner).run(request);
        self.last_action_feedback = Some(action_feedback(&result));
        Some(result)
    }
}

fn action_request(
    action_id: ActionId,
    confirmed: bool,
    panel: &TmuxManagerPanelState,
    snapshot: &TmuxManagerSnapshot,
) -> TmuxActionRequest {
    let mut request = TmuxActionRequest::new(action_id)
        .confirmed(confirmed)
        .active_tmux(snapshot.current.is_some());
    if let Some(target) = action_target(action_id, panel, snapshot) {
        request = request.target(target);
    }
    request
}

fn action_target(
    action_id: ActionId,
    panel: &TmuxManagerPanelState,
    snapshot: &TmuxManagerSnapshot,
) -> Option<String> {
    match action_id {
        ActionId::StartSession
        | ActionId::AttachSession
        | ActionId::RenameSession
        | ActionId::KillSession => panel.selected_session_name(snapshot).map(str::to_owned),
        ActionId::SplitPaneRight
        | ActionId::SplitPaneDown
        | ActionId::SelectPane
        | ActionId::KillPane => panel.selected_pane_id(snapshot).map(str::to_owned),
        ActionId::NewWindow | ActionId::RenameWindow | ActionId::KillWindow => {
            selected_window_target(panel, snapshot)
        }
        ActionId::DetachSession
        | ActionId::NextWindow
        | ActionId::PreviousWindow
        | ActionId::ZoomPane
        | ActionId::ShowHelp => None,
    }
}

fn selected_window_target(
    panel: &TmuxManagerPanelState,
    snapshot: &TmuxManagerSnapshot,
) -> Option<String> {
    selected_windows(snapshot, panel.selected_session)
        .get(panel.selected_window)
        .map(|window| format!("{}:{}", window.session_name, window.index))
}

fn action_feedback(result: &TmuxActionResult) -> String {
    match result {
        TmuxActionResult::Success { action_id, .. } => {
            format!("{} success", stable_id(*action_id))
        }
        TmuxActionResult::CommandFailed {
            action_id, failure, ..
        } => {
            let stderr = failure.stderr.trim();
            if stderr.is_empty() {
                return format!("{} failed", stable_id(*action_id));
            }
            format!("{} failed: {stderr}", stable_id(*action_id))
        }
        TmuxActionResult::ConfirmationRequired { action_id, .. } => {
            format!("{} requires confirmation", stable_id(*action_id))
        }
        TmuxActionResult::NoActiveSession { action_id, .. } => {
            format!("{} needs active tmux", stable_id(*action_id))
        }
        TmuxActionResult::TmuxMissing { action_id, .. } => {
            format!("{} failed: tmux missing", stable_id(*action_id))
        }
        TmuxActionResult::Skipped {
            action_id, reason, ..
        } => {
            format!("{} skipped: {reason}", stable_id(*action_id))
        }
    }
}

fn stable_id(action_id: ActionId) -> &'static str {
    TmuxAction::by_id(action_id)
        .map(|action| action.stable_id)
        .unwrap_or("tmux-action")
}
