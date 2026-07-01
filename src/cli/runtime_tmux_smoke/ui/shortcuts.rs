//! Shortcut dispatch proof helpers for the native tmux UI smoke.

use winit::keyboard::{Key, ModifiersState};

use crate::app::{NativeTerminalRuntime, TmuxManagerKeyOutcome};
use crate::tmux::{ActionId, SocketTmuxCommandRunner, TmuxActionResult};

pub(super) fn drive_shortcut_action(
    runtime: &mut NativeTerminalRuntime<()>,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    let requested =
        runtime.handle_tmux_manager_key(&Key::Character("c".into()), ModifiersState::empty());
    matches!(
        runtime.dispatch_tmux_manager_action(requested, runner),
        Some(TmuxActionResult::Success {
            action_id: ActionId::NewWindow,
            ..
        })
    )
}

pub(super) fn drive_destructive_shortcut_confirmation(
    runtime: &mut NativeTerminalRuntime<()>,
) -> bool {
    let confirmation =
        runtime.handle_tmux_manager_key(&Key::Character("q".into()), ModifiersState::empty());
    let canceled = runtime.handle_tmux_manager_key(
        &Key::Named(winit::keyboard::NamedKey::Escape),
        ModifiersState::empty(),
    );
    matches!(
        confirmation,
        TmuxManagerKeyOutcome::ConfirmationRequired(ActionId::KillSession)
    ) && matches!(canceled, TmuxManagerKeyOutcome::Consumed)
}

pub(super) fn drive_refresh_shortcut(runtime: &mut NativeTerminalRuntime<()>) -> bool {
    matches!(
        runtime.handle_tmux_manager_key(&Key::Character("r".into()), ModifiersState::empty()),
        TmuxManagerKeyOutcome::RefreshRequested
    )
}
