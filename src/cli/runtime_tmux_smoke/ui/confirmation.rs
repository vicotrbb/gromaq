//! Confirmation and default action proof helpers for the native tmux UI smoke.

use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::app::TmuxManagerKeyOutcome;
use crate::tmux::{ActionId, SocketTmuxCommandRunner, TmuxActionResult};

pub(super) fn drive_confirmation_cancel(runtime: &mut super::SmokeRuntime) -> bool {
    for key in [
        Key::Named(NamedKey::ArrowRight),
        Key::Named(NamedKey::ArrowRight),
        Key::Named(NamedKey::ArrowRight),
        Key::Named(NamedKey::ArrowRight),
        Key::Named(NamedKey::ArrowDown),
    ] {
        if matches!(
            runtime.handle_tmux_manager_key(&key, ModifiersState::empty()),
            TmuxManagerKeyOutcome::Ignored
        ) {
            return false;
        }
    }
    let confirmation =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty());
    let canceled =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Escape), ModifiersState::empty());
    matches!(
        confirmation,
        TmuxManagerKeyOutcome::ConfirmationRequired(ActionId::KillWindow)
    ) && matches!(canceled, TmuxManagerKeyOutcome::Consumed)
}

pub(super) fn drive_safe_action(
    runtime: &mut super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    let previous_action =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::ArrowUp), ModifiersState::empty());
    let requested =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty());
    let result = runtime.dispatch_tmux_manager_action(requested, runner);
    !matches!(previous_action, TmuxManagerKeyOutcome::Ignored)
        && matches!(
            result,
            Some(TmuxActionResult::Success {
                action_id: ActionId::SplitPaneRight,
                ..
            })
        )
}
