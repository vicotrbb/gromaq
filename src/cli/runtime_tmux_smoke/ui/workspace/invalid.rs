//! Invalid workspace preflight proof for the native tmux UI smoke.

use crate::app::{TmuxManagerKeyOutcome, TmuxManagerPanelState, TmuxWorkspaceUiPreset};
use crate::config::TmuxWorkspaceSettings;
use crate::renderer::WgpuRenderer;
use crate::tmux::TmuxManagerSnapshot;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use super::super::render::render_manager_panel_contains;

pub(super) fn drive_invalid_workspace_preflight(
    snapshot: &TmuxManagerSnapshot,
    renderer: &mut WgpuRenderer,
) -> bool {
    let mut panel = TmuxManagerPanelState::open_for_snapshot_with_workspaces(
        snapshot,
        vec![TmuxWorkspaceUiPreset::new(
            "invalid-ui-smoke",
            TmuxWorkspaceSettings::default(),
        )],
    );
    panel.focus_next();
    panel.focus_next();
    panel.focus_next();
    let blocked = panel.handle_key(
        &Key::Named(NamedKey::Enter),
        ModifiersState::empty(),
        snapshot,
    ) == TmuxManagerKeyOutcome::Consumed
        && panel
            .workspace_feedback()
            .is_some_and(|feedback| feedback.contains("session is empty"));
    let Ok(mut runtime) = super::super::smoke_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(
        snapshot.clone(),
        vec![TmuxWorkspaceUiPreset::new(
            "invalid-ui-smoke",
            TmuxWorkspaceSettings::default(),
        )],
    );
    blocked
        && render_manager_panel_contains(
            &mut runtime,
            renderer,
            "invalid-ui-smoke*invalid:sessionisempty",
        )
}
