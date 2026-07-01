//! Zoom shortcut proof helpers for the native tmux UI smoke.

use winit::keyboard::{Key, ModifiersState};

use crate::tmux::{ActionId, SocketTmuxCommandRunner, TmuxActionResult};

pub(in crate::cli::runtime_tmux_smoke::ui) fn drive_zoom_shortcut(
    runtime: &mut super::super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    dispatch_zoom(runtime, runner)
}

pub(in crate::cli::runtime_tmux_smoke::ui) fn drive_zoom_toggle_shortcut(
    runtime: &mut super::super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    dispatch_zoom(runtime, runner) && dispatch_zoom(runtime, runner)
}

fn dispatch_zoom(
    runtime: &mut super::super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    let requested =
        runtime.handle_tmux_manager_key(&Key::Character("z".into()), ModifiersState::empty());
    matches!(
        runtime.dispatch_tmux_manager_action(requested, runner),
        Some(TmuxActionResult::Success {
            action_id: ActionId::ZoomPane,
            ..
        })
    )
}
