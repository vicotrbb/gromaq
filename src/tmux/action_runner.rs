//! tmux action execution with confirmation and teaching hints.

mod commands;

use commands::command_args;

use super::{ActionId, SystemTmuxCommandRunner, TmuxAction, TmuxCommandFailure};
use super::{TmuxCommandRunner, TmuxError};

/// Request to run a tmux action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxActionRequest {
    /// Action to execute.
    pub action_id: ActionId,
    /// Optional tmux target such as session, window, or pane id.
    pub target: Option<String>,
    /// Optional replacement name for rename actions.
    pub new_name: Option<String>,
    /// Whether a destructive action was explicitly confirmed.
    pub confirmed: bool,
    /// Whether the caller has an active tmux session context.
    pub active_tmux: bool,
}

impl TmuxActionRequest {
    /// Build a request for an action.
    pub fn new(action_id: ActionId) -> Self {
        Self {
            action_id,
            target: None,
            new_name: None,
            confirmed: false,
            active_tmux: true,
        }
    }

    /// Add a tmux target.
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Add a replacement name for rename actions.
    pub fn new_name(mut self, new_name: impl Into<String>) -> Self {
        self.new_name = Some(new_name.into());
        self
    }

    /// Mark the action as explicitly confirmed.
    pub fn confirmed(mut self, confirmed: bool) -> Self {
        self.confirmed = confirmed;
        self
    }

    /// Set whether the action has an active tmux session context.
    pub fn active_tmux(mut self, active_tmux: bool) -> Self {
        self.active_tmux = active_tmux;
        self
    }
}

/// Result of attempting a tmux action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmuxActionResult {
    /// The action ran successfully.
    Success {
        /// Action that ran.
        action_id: ActionId,
        /// User-facing tmux command and key hint.
        teaching_hint: String,
    },
    /// The action was skipped before running tmux.
    Skipped {
        /// Action that was skipped.
        action_id: ActionId,
        /// Reason the action did not run.
        reason: String,
        /// User-facing tmux command and key hint.
        teaching_hint: String,
    },
    /// The action is destructive and needs confirmation.
    ConfirmationRequired {
        /// Destructive action requiring confirmation.
        action_id: ActionId,
        /// User-facing tmux command and key hint.
        teaching_hint: String,
    },
    /// The action needs an active tmux session and no target was provided.
    NoActiveSession {
        /// Action that could not run.
        action_id: ActionId,
        /// User-facing tmux command and key hint.
        teaching_hint: String,
    },
    /// tmux was not found.
    TmuxMissing {
        /// Action that could not run.
        action_id: ActionId,
        /// User-facing tmux command and key hint.
        teaching_hint: String,
    },
    /// tmux returned an unsuccessful status.
    CommandFailed {
        /// Action that failed.
        action_id: ActionId,
        /// Command failure details.
        failure: TmuxCommandFailure,
        /// User-facing tmux command and key hint.
        teaching_hint: String,
    },
}

/// Executes tmux actions through a command runner.
#[derive(Debug, Clone)]
pub struct TmuxActionRunner<R = SystemTmuxCommandRunner> {
    runner: R,
}

impl<R> TmuxActionRunner<R>
where
    R: TmuxCommandRunner,
{
    /// Create an action runner.
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    /// Run a tmux action request.
    pub fn run(&self, request: TmuxActionRequest) -> TmuxActionResult {
        let action = TmuxAction::by_id(request.action_id).expect("registered tmux action");
        let teaching_hint = teaching_hint(action);
        if action.confirmation_required && !request.confirmed {
            return TmuxActionResult::ConfirmationRequired {
                action_id: action.id,
                teaching_hint,
            };
        }
        if action.requires_active_tmux
            && !action.can_run_outside_tmux
            && !request.active_tmux
            && request.target.is_none()
        {
            return TmuxActionResult::NoActiveSession {
                action_id: action.id,
                teaching_hint,
            };
        }
        let args = match command_args(&request) {
            Ok(args) => args,
            Err(reason) => {
                return TmuxActionResult::Skipped {
                    action_id: action.id,
                    reason,
                    teaching_hint,
                };
            }
        };
        let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        match self.runner.run_tmux(&arg_refs) {
            Ok(_) => TmuxActionResult::Success {
                action_id: action.id,
                teaching_hint,
            },
            Err(TmuxError::Missing) => TmuxActionResult::TmuxMissing {
                action_id: action.id,
                teaching_hint,
            },
            Err(TmuxError::Command(failure)) => TmuxActionResult::CommandFailed {
                action_id: action.id,
                failure,
                teaching_hint,
            },
            Err(error) => TmuxActionResult::Skipped {
                action_id: action.id,
                reason: format!("{error:?}"),
                teaching_hint,
            },
        }
    }
}

fn teaching_hint(action: &TmuxAction) -> String {
    match action.key_binding {
        Some(key) => format!("tmux command: {}\ntmux key: {key}", action.tmux_command),
        None => format!("tmux command: {}", action.tmux_command),
    }
}
