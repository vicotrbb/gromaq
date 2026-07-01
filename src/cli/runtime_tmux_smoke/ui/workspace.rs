//! Workspace launcher proof helpers for the native tmux UI smoke.

use crate::app::{TmuxManagerKeyOutcome, TmuxManagerPanelState, TmuxWorkspaceUiPreset};
use crate::config::{TmuxWorkspaceSettings, TmuxWorkspaceWindowSettings};
use crate::renderer::WgpuRenderer;
use crate::tmux::{
    SocketTmuxCommandRunner, TmuxCommandFailure, TmuxCommandOutput, TmuxCommandRunner, TmuxError,
    TmuxManagerSnapshot, TmuxStateReader, TmuxWorkspaceResult,
};

use super::render::render_manager_panel_contains;

const UI_WORKSPACE_SESSION: &str = "gromaq-runtime-tmux-ui-workspace";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WorkspaceProof {
    pub(super) started: bool,
    pub(super) feedback_checked: bool,
    pub(super) existing_attach_checked: bool,
    pub(super) duplicate_prevented: bool,
    pub(super) failure_feedback_checked: bool,
}

pub(super) fn run_workspace_proof(
    snapshot: &TmuxManagerSnapshot,
    preset: &TmuxWorkspaceUiPreset,
    runner: &SocketTmuxCommandRunner,
    renderer: &mut WgpuRenderer,
) -> WorkspaceProof {
    let mut panel =
        TmuxManagerPanelState::open_for_snapshot_with_workspaces(snapshot, vec![preset.clone()]);
    let started = matches!(
        panel.launch_selected_workspace(runner),
        Some(Ok(TmuxWorkspaceResult::Started { .. }))
    );
    let feedback_checked = panel
        .workspace_feedback()
        .is_some_and(|feedback| feedback.contains("workspace gromaq-ui-smoke started session"));
    let before = session_count(runner, UI_WORKSPACE_SESSION);
    let _ = panel.launch_selected_workspace(runner);
    let after = session_count(runner, UI_WORKSPACE_SESSION);
    let existing_attach_checked = drive_workspace_existing_attach(snapshot, preset, runner);
    let failure_feedback_checked = drive_workspace_failure_feedback(snapshot, runner, renderer);
    WorkspaceProof {
        started,
        feedback_checked,
        existing_attach_checked,
        duplicate_prevented: before == Some(1) && after == Some(1),
        failure_feedback_checked,
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

pub(super) fn docs_workspace_preset() -> TmuxWorkspaceUiPreset {
    TmuxWorkspaceUiPreset::new(
        "docs-ui-smoke",
        TmuxWorkspaceSettings {
            session: "gromaq-runtime-tmux-ui-docs".to_owned(),
            root: None,
            windows: vec![TmuxWorkspaceWindowSettings {
                name: "docs".to_owned(),
                panes: vec!["sleep 60".to_owned()],
            }],
        },
    )
}

fn drive_workspace_failure_feedback(
    snapshot: &TmuxManagerSnapshot,
    _runner: &SocketTmuxCommandRunner,
    renderer: &mut WgpuRenderer,
) -> bool {
    let Ok(mut runtime) = super::smoke_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(
        snapshot.clone(),
        vec![failing_workspace_preset()],
    );
    let runner = WorkspaceFailureRunner;
    let Some(result) = runtime
        .dispatch_tmux_manager_workspace(TmuxManagerKeyOutcome::WorkspaceLaunchRequested, &runner)
    else {
        return false;
    };
    matches!(result, Err(TmuxError::Command(_)))
        && render_manager_panel_contains(&mut runtime, renderer, "workspacegromaq-ui-failfailed")
}

fn drive_workspace_existing_attach(
    snapshot: &TmuxManagerSnapshot,
    preset: &TmuxWorkspaceUiPreset,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    let Ok(mut runtime) = super::smoke_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(snapshot.clone(), vec![preset.clone()]);
    let before = runtime.dump_runtime_perf_metrics().pty_input_writes;
    let result = runtime
        .dispatch_tmux_manager_workspace(TmuxManagerKeyOutcome::WorkspaceLaunchRequested, runner);
    let after = runtime.dump_runtime_perf_metrics().pty_input_writes;
    matches!(
        result,
        Some(Ok(TmuxWorkspaceResult::Existing { session }))
            if session == UI_WORKSPACE_SESSION
    ) && after == before + 1
}

fn failing_workspace_preset() -> TmuxWorkspaceUiPreset {
    TmuxWorkspaceUiPreset::new(
        "gromaq-ui-fail",
        TmuxWorkspaceSettings {
            session: "gromaq-runtime-tmux-ui-fail".to_owned(),
            root: None,
            windows: vec![TmuxWorkspaceWindowSettings {
                name: "fail".to_owned(),
                panes: vec!["sleep 60".to_owned()],
            }],
        },
    )
}

#[derive(Debug, Clone, Copy)]
struct WorkspaceFailureRunner;

impl TmuxCommandRunner for WorkspaceFailureRunner {
    fn run_tmux(&self, args: &[&str]) -> Result<TmuxCommandOutput, TmuxError> {
        if args.first() == Some(&"has-session") {
            return Err(TmuxError::Command(TmuxCommandFailure::new(
                args.iter().map(|arg| (*arg).to_owned()).collect(),
                1,
                "can't find session".to_owned(),
            )));
        }
        Err(TmuxError::Command(TmuxCommandFailure::new(
            args.iter().map(|arg| (*arg).to_owned()).collect(),
            1,
            "workspace smoke forced failure".to_owned(),
        )))
    }
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
