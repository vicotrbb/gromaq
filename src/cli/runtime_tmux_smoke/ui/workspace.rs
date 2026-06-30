//! Workspace launcher proof helpers for the native tmux UI smoke.

use crate::app::{TmuxManagerPanelState, TmuxWorkspaceUiPreset};
use crate::config::{TmuxWorkspaceSettings, TmuxWorkspaceWindowSettings};
use crate::tmux::{
    SocketTmuxCommandRunner, TmuxManagerSnapshot, TmuxStateReader, TmuxWorkspaceResult,
};

const UI_WORKSPACE_SESSION: &str = "gromaq-runtime-tmux-ui-workspace";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WorkspaceProof {
    pub(super) started: bool,
    pub(super) duplicate_prevented: bool,
}

pub(super) fn run_workspace_proof(
    snapshot: &TmuxManagerSnapshot,
    preset: &TmuxWorkspaceUiPreset,
    runner: &SocketTmuxCommandRunner,
) -> WorkspaceProof {
    let mut panel =
        TmuxManagerPanelState::open_for_snapshot_with_workspaces(snapshot, vec![preset.clone()]);
    let started = matches!(
        panel.launch_selected_workspace(runner),
        Some(Ok(TmuxWorkspaceResult::Started { .. }))
    );
    let before = session_count(runner, UI_WORKSPACE_SESSION);
    let _ = panel.launch_selected_workspace(runner);
    let after = session_count(runner, UI_WORKSPACE_SESSION);
    WorkspaceProof {
        started,
        duplicate_prevented: before == Some(1) && after == Some(1),
    }
}

pub(super) fn workspace_preset() -> TmuxWorkspaceUiPreset {
    TmuxWorkspaceUiPreset::new(
        "gromaq-ui-smoke",
        TmuxWorkspaceSettings {
            session: UI_WORKSPACE_SESSION.to_owned(),
            root: None,
            windows: vec![TmuxWorkspaceWindowSettings {
                name: "code".to_owned(),
                panes: vec!["sleep 60".to_owned()],
            }],
        },
    )
}

fn session_count(runner: &SocketTmuxCommandRunner, name: &str) -> Option<usize> {
    let state = TmuxStateReader::new(runner.clone()).read_state().ok()?;
    Some(
        state
            .sessions
            .iter()
            .filter(|session| session.name == name)
            .count(),
    )
}
