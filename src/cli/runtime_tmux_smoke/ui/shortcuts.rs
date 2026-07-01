//! Shortcut dispatch proof helpers for the native tmux UI smoke.

use winit::keyboard::{Key, ModifiersState};

use crate::app::TmuxManagerKeyOutcome;
use crate::tmux::{ActionId, SocketTmuxCommandRunner, TmuxActionResult, TmuxManagerSnapshot};

pub(super) fn drive_shortcut_action(
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

pub(super) fn drive_split_down_shortcut(
    runtime: &mut super::SmokeRuntime,
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

pub(super) fn drive_zoom_shortcut(
    runtime: &mut super::SmokeRuntime,
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

pub(super) fn drive_rename_window_action(
    runtime: &mut super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    if !matches!(
        runtime.handle_tmux_manager_key(&Key::Character("e".into()), ModifiersState::empty()),
        TmuxManagerKeyOutcome::Consumed
    ) {
        return false;
    }
    for character in "ui-smoke-renamed".chars() {
        if !matches!(
            runtime.handle_tmux_manager_key(
                &Key::Character(character.to_string().into()),
                ModifiersState::empty()
            ),
            TmuxManagerKeyOutcome::Consumed
        ) {
            return false;
        }
    }
    let requested = runtime.handle_tmux_manager_key(
        &Key::Named(winit::keyboard::NamedKey::Enter),
        ModifiersState::empty(),
    );
    matches!(
        runtime.dispatch_tmux_manager_action(requested, runner),
        Some(TmuxActionResult::Success {
            action_id: ActionId::RenameWindow,
            ..
        })
    )
}

pub(super) fn drive_name_entry_action(
    runtime: &mut super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    for _ in 0..3 {
        if matches!(
            runtime.handle_tmux_manager_key(
                &Key::Named(winit::keyboard::NamedKey::ArrowDown),
                ModifiersState::empty()
            ),
            TmuxManagerKeyOutcome::Ignored
        ) {
            return false;
        }
    }
    if !matches!(
        runtime.handle_tmux_manager_key(
            &Key::Named(winit::keyboard::NamedKey::Enter),
            ModifiersState::empty()
        ),
        TmuxManagerKeyOutcome::Consumed
    ) {
        return false;
    }
    for character in "gromaq-runtime-tmux-ui-name".chars() {
        if !matches!(
            runtime.handle_tmux_manager_key(
                &Key::Character(character.to_string().into()),
                ModifiersState::empty()
            ),
            TmuxManagerKeyOutcome::Consumed
        ) {
            return false;
        }
    }
    let requested = runtime.handle_tmux_manager_key(
        &Key::Named(winit::keyboard::NamedKey::Enter),
        ModifiersState::empty(),
    );
    matches!(
        runtime.dispatch_tmux_manager_action(requested, runner),
        Some(TmuxActionResult::Success {
            action_id: ActionId::StartSession,
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
