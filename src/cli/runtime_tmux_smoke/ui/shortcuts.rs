//! Shortcut dispatch proof helpers for the native tmux UI smoke.

mod missing;
mod name_entry;
mod split;
mod zoom;

use winit::keyboard::{Key, ModifiersState};

use crate::app::TmuxManagerKeyOutcome;
use crate::tmux::{ActionId, SocketTmuxCommandRunner, TmuxActionResult, TmuxManagerSnapshot};

pub(super) use missing::drive_missing_tmux_feedback;
pub(super) use name_entry::{
    drive_name_entry_action, drive_rename_session_action, drive_rename_window_action,
    drive_start_session_feedback, drive_start_session_pty_handoff,
};
pub(super) use split::{drive_split_down_shortcut, drive_split_right_shortcut};
pub(super) use zoom::{drive_zoom_shortcut, drive_zoom_toggle_shortcut};

pub(super) fn drive_new_window_shortcut(
    runtime: &mut super::SmokeRuntime,
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

pub(super) fn drive_attach_session_handoff(runtime: &mut super::SmokeRuntime) -> bool {
    let before = runtime.dump_runtime_perf_metrics().pty_input_writes;
    let requested =
        runtime.handle_tmux_manager_key(&Key::Character("a".into()), ModifiersState::empty());
    let result = runtime.dispatch_tmux_manager_terminal_action(requested);
    let after = runtime.dump_runtime_perf_metrics().pty_input_writes;
    matches!(
        result,
        Some(TmuxActionResult::Success {
            action_id: ActionId::AttachSession,
            ..
        })
    ) && after == before + 1
}

pub(super) fn drive_detach_session_failure(
    runtime: &mut super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    let requested =
        runtime.handle_tmux_manager_key(&Key::Character("d".into()), ModifiersState::empty());
    matches!(
        runtime.dispatch_tmux_manager_action(requested, runner),
        Some(TmuxActionResult::CommandFailed {
            action_id: ActionId::DetachSession,
            ..
        })
    )
}

pub(super) fn drive_select_pane_shortcut(
    runtime: &mut super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    let requested =
        runtime.handle_tmux_manager_key(&Key::Character("m".into()), ModifiersState::empty());
    matches!(
        runtime.dispatch_tmux_manager_action(requested, runner),
        Some(TmuxActionResult::Success {
            action_id: ActionId::SelectPane,
            ..
        })
    )
}

pub(super) fn drive_window_cycle_shortcuts(
    runtime: &mut super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    let next =
        runtime.handle_tmux_manager_key(&Key::Character("n".into()), ModifiersState::empty());
    let next_result = runtime.dispatch_tmux_manager_action(next, runner);
    let previous =
        runtime.handle_tmux_manager_key(&Key::Character("p".into()), ModifiersState::empty());
    let previous_result = runtime.dispatch_tmux_manager_action(previous, runner);
    matches!(
        next_result,
        Some(TmuxActionResult::Success {
            action_id: ActionId::NextWindow,
            ..
        })
    ) && matches!(
        previous_result,
        Some(TmuxActionResult::Success {
            action_id: ActionId::PreviousWindow,
            ..
        })
    )
}

pub(super) fn drive_destructive_shortcut_confirmation(runtime: &mut super::SmokeRuntime) -> bool {
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

pub(super) fn drive_kill_pane_confirmation(
    runtime: &mut super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    let confirmation =
        runtime.handle_tmux_manager_key(&Key::Character("x".into()), ModifiersState::empty());
    if !matches!(
        confirmation,
        TmuxManagerKeyOutcome::ConfirmationRequired(ActionId::KillPane)
    ) {
        return false;
    }
    let confirmed =
        runtime.handle_tmux_manager_key(&Key::Character("y".into()), ModifiersState::empty());
    matches!(
        runtime.dispatch_tmux_manager_action(confirmed, runner),
        Some(TmuxActionResult::Success {
            action_id: ActionId::KillPane,
            ..
        })
    )
}

pub(super) fn drive_kill_window_confirmation(
    runtime: &mut super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    let confirmation =
        runtime.handle_tmux_manager_key(&Key::Character("w".into()), ModifiersState::empty());
    if !matches!(
        confirmation,
        TmuxManagerKeyOutcome::ConfirmationRequired(ActionId::KillWindow)
    ) {
        return false;
    }
    let confirmed =
        runtime.handle_tmux_manager_key(&Key::Character("y".into()), ModifiersState::empty());
    matches!(
        runtime.dispatch_tmux_manager_action(confirmed, runner),
        Some(TmuxActionResult::Success {
            action_id: ActionId::KillWindow,
            ..
        })
    )
}

pub(super) fn drive_refresh_shortcut(runtime: &mut super::SmokeRuntime) -> bool {
    matches!(
        runtime.handle_tmux_manager_key(&Key::Character("r".into()), ModifiersState::empty()),
        TmuxManagerKeyOutcome::RefreshRequested
    )
}

pub(super) fn drive_unavailable_shortcut_block(runtime: &mut super::SmokeRuntime) -> bool {
    if runtime.tmux_manager_panel_is_open() {
        let _ = runtime.handle_tmux_manager_key(
            &Key::Named(winit::keyboard::NamedKey::Escape),
            ModifiersState::empty(),
        );
    }
    runtime.toggle_tmux_manager_panel(TmuxManagerSnapshot::no_server());
    matches!(
        runtime.handle_tmux_manager_key(&Key::Character("s".into()), ModifiersState::empty()),
        TmuxManagerKeyOutcome::Consumed
    )
}
