//! Name-entry shortcut proof helpers for the native tmux UI smoke.

use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::app::TmuxManagerKeyOutcome;
use crate::tmux::{ActionId, SocketTmuxCommandRunner, TmuxActionResult, TmuxCommandRunner};

pub(in crate::cli::runtime_tmux_smoke::ui) fn drive_rename_window_action(
    runtime: &mut super::super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    if !start_name_entry(runtime, "e") {
        return false;
    }
    type_name(runtime, "ui-smoke-renamed")
        && dispatch_name_entry(runtime, runner, ActionId::RenameWindow)
}

pub(in crate::cli::runtime_tmux_smoke::ui) fn drive_rename_session_action(
    runtime: &mut super::super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
    original: &str,
    renamed: &str,
) -> bool {
    if !start_name_entry(runtime, "u") || !type_name(runtime, renamed) {
        return false;
    }
    let renamed_through_ui = dispatch_name_entry(runtime, runner, ActionId::RenameSession);
    renamed_through_ui
        && runner
            .run_tmux(&["rename-session", "-t", renamed, original])
            .is_ok()
}

pub(in crate::cli::runtime_tmux_smoke::ui) fn drive_name_entry_action(
    runtime: &mut super::super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    for _ in 0..3 {
        if matches!(
            runtime
                .handle_tmux_manager_key(&Key::Named(NamedKey::ArrowDown), ModifiersState::empty()),
            TmuxManagerKeyOutcome::Ignored
        ) {
            return false;
        }
    }
    if !matches!(
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()),
        TmuxManagerKeyOutcome::Consumed
    ) {
        return false;
    }
    type_name(runtime, "gromaq-runtime-tmux-ui-name")
        && dispatch_name_entry(runtime, runner, ActionId::StartSession)
}

fn start_name_entry(runtime: &mut super::super::SmokeRuntime, shortcut: &str) -> bool {
    matches!(
        runtime.handle_tmux_manager_key(&Key::Character(shortcut.into()), ModifiersState::empty()),
        TmuxManagerKeyOutcome::Consumed
    )
}

fn type_name(runtime: &mut super::super::SmokeRuntime, name: &str) -> bool {
    name.chars().all(|character| {
        matches!(
            runtime.handle_tmux_manager_key(
                &Key::Character(character.to_string().into()),
                ModifiersState::empty()
            ),
            TmuxManagerKeyOutcome::Consumed
        )
    })
}

fn dispatch_name_entry(
    runtime: &mut super::super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
    action_id: ActionId,
) -> bool {
    let requested =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty());
    matches!(
        runtime.dispatch_tmux_manager_action(requested, runner),
        Some(TmuxActionResult::Success {
            action_id: dispatched,
            ..
        }) if dispatched == action_id
    )
}
