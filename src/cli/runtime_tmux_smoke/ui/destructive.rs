//! Destructive action proof helpers for the native tmux UI smoke.

use winit::keyboard::{Key, ModifiersState};

use crate::app::TmuxManagerKeyOutcome;
use crate::tmux::{
    ActionId, SocketTmuxCommandRunner, TmuxActionResult, TmuxCommandRunner, TmuxManager,
    TmuxManagerCurrent,
};

const KILL_SESSION: &str = "gromaq-runtime-tmux-ui-kill";

pub(super) fn drive_kill_session_confirmation(
    runtime: &mut super::SmokeRuntime,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    if runner
        .run_tmux(&["new-session", "-d", "-s", KILL_SESSION])
        .is_err()
    {
        return false;
    }
    let Ok(mut snapshot) = TmuxManager::new(runner.clone()).snapshot() else {
        return false;
    };
    let Some(pane_id) = snapshot
        .state
        .panes
        .iter()
        .find(|pane| pane.session_name == KILL_SESSION)
        .map(|pane| pane.id.clone())
    else {
        return false;
    };
    snapshot.current = Some(TmuxManagerCurrent {
        session_name: KILL_SESSION.to_owned(),
        window_index: 0,
        pane_id,
    });
    runtime.refresh_tmux_manager_panel(snapshot);

    let confirmation =
        runtime.handle_tmux_manager_key(&Key::Character("q".into()), ModifiersState::empty());
    if !matches!(
        confirmation,
        TmuxManagerKeyOutcome::ConfirmationRequired(ActionId::KillSession)
    ) {
        return false;
    }
    let confirmed =
        runtime.handle_tmux_manager_key(&Key::Character("y".into()), ModifiersState::empty());
    matches!(
        runtime.dispatch_tmux_manager_action(confirmed, runner),
        Some(TmuxActionResult::Success {
            action_id: ActionId::KillSession,
            ..
        })
    )
}
