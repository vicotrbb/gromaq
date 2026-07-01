//! Split shortcut proof helpers for the native tmux UI smoke.

use winit::keyboard::{Key, ModifiersState};

use crate::tmux::{ActionId, SocketTmuxCommandRunner, TmuxActionResult};

pub(in crate::cli::runtime_tmux_smoke::ui) fn drive_split_down_shortcut(
    runtime: &mut super::super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    let requested =
        runtime.handle_tmux_manager_key(&Key::Character("v".into()), ModifiersState::empty());
    matches!(
        runtime.dispatch_tmux_manager_action(requested, runner),
        Some(TmuxActionResult::Success {
            action_id: ActionId::SplitPaneDown,
            ..
        })
    )
}

pub(in crate::cli::runtime_tmux_smoke::ui) fn drive_split_right_shortcut(
    runtime: &mut super::super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    let requested =
        runtime.handle_tmux_manager_key(&Key::Character("s".into()), ModifiersState::empty());
    matches!(
        runtime.dispatch_tmux_manager_action(requested, runner),
        Some(TmuxActionResult::Success {
            action_id: ActionId::SplitPaneRight,
            ..
        })
    )
}
