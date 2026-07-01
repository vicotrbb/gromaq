//! No-shell PTY handoff proof helpers for the native tmux UI smoke.

use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::app::{NativeTerminalRuntimeConfig, TmuxManagerKeyOutcome};
use crate::renderer::WgpuRenderer;
use crate::tmux::{
    ActionId, SocketTmuxCommandRunner, TmuxActionResult, TmuxManagerSnapshot, TmuxManagerStatus,
    TmuxState, TmuxWorkspaceResult,
};

use super::render::render_manager_panel_contains;
use super::workspace::workspace_preset;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SkippedPtyHandoffProof {
    pub(super) attach: bool,
    pub(super) start: bool,
    pub(super) workspace: bool,
}

pub(super) fn drive(
    snapshot: &TmuxManagerSnapshot,
    runner: &SocketTmuxCommandRunner,
    renderer: &mut WgpuRenderer,
) -> SkippedPtyHandoffProof {
    SkippedPtyHandoffProof {
        attach: drive_attach_session_skipped(snapshot, renderer),
        start: drive_start_session_skipped(runner, renderer),
        workspace: drive_workspace_attach_skipped(snapshot, runner, renderer),
    }
}

fn drive_attach_session_skipped(
    snapshot: &TmuxManagerSnapshot,
    renderer: &mut WgpuRenderer,
) -> bool {
    let Ok(mut runtime) = no_shell_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(snapshot.clone(), Vec::new());
    let outcome =
        runtime.handle_tmux_manager_key(&Key::Character("a".into()), ModifiersState::empty());
    matches!(
        runtime.dispatch_tmux_manager_terminal_action(outcome),
        Some(TmuxActionResult::Skipped {
            action_id: ActionId::AttachSession,
            ..
        })
    ) && render_manager_panel_contains(
        &mut runtime,
        renderer,
        "attach-sessionskipped:shellnotstarted",
    )
}

fn drive_start_session_skipped(
    runner: &SocketTmuxCommandRunner,
    renderer: &mut WgpuRenderer,
) -> bool {
    let Ok(mut runtime) = no_shell_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(empty_snapshot(), Vec::new());
    if !enter_start_session_name(&mut runtime, "gromaq-runtime-tmux-ui-skip-start") {
        return false;
    }
    let outcome =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty());
    matches!(
        runtime.dispatch_tmux_manager_action(outcome, runner),
        Some(TmuxActionResult::Success {
            action_id: ActionId::StartSession,
            ..
        })
    ) && render_manager_panel_contains(&mut runtime, renderer, "attachskipped:shellnotstarted")
}

fn drive_workspace_attach_skipped(
    snapshot: &TmuxManagerSnapshot,
    runner: &SocketTmuxCommandRunner,
    renderer: &mut WgpuRenderer,
) -> bool {
    let Ok(mut runtime) = no_shell_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(snapshot.clone(), vec![workspace_preset()]);
    matches!(
        runtime.dispatch_tmux_manager_workspace(
            TmuxManagerKeyOutcome::WorkspaceLaunchRequested,
            runner
        ),
        Some(Ok(
            TmuxWorkspaceResult::Existing { .. } | TmuxWorkspaceResult::Started { .. }
        ))
    ) && render_manager_panel_contains(&mut runtime, renderer, "attachskipped:shellnotstarted")
}

fn enter_start_session_name(runtime: &mut super::SmokeRuntime, name: &str) -> bool {
    if !matches!(
        runtime.handle_tmux_manager_key(&Key::Character("t".into()), ModifiersState::empty()),
        TmuxManagerKeyOutcome::Consumed
    ) {
        return false;
    }
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

fn no_shell_runtime() -> Result<super::SmokeRuntime, String> {
    let mut runtime = super::SmokeRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 220,
        terminal_rows: 10,
        ..NativeTerminalRuntimeConfig::default()
    })
    .map_err(|error| error.to_string())?;
    runtime
        .write_startup_text("gromaq tmux no-shell smoke\r\n> ")
        .map_err(|error| error.to_string())?;
    Ok(runtime)
}

fn empty_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState::default(),
        current: None,
    }
}
