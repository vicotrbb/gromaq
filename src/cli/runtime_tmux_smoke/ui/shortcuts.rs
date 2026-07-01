//! Shortcut dispatch proof helpers for the native tmux UI smoke.

use winit::keyboard::{Key, ModifiersState};

use crate::app::NativeTerminalRuntime;
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
