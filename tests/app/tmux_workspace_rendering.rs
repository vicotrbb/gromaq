use gromaq::app::{
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerFocus, TmuxManagerKeyOutcome,
    TmuxManagerPanelState,
};
use gromaq::config::TmuxWorkspaceSettings;
use gromaq::tmux::{TmuxManagerSnapshot, TmuxManagerStatus, TmuxState};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::support::{MockFrameRenderer, MockPtySession};

#[test]
fn tmux_manager_panel_marks_invalid_workspace_preset_before_launch() {
    let snapshot = TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState::default(),
        current: None,
    };
    let panel = TmuxManagerPanelState::open_for_snapshot_with_workspaces(
        &snapshot,
        vec![gromaq::app::TmuxWorkspaceUiPreset::new(
            "bad",
            TmuxWorkspaceSettings::default(),
        )],
    );
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 140,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &panel)
            .unwrap()
    );

    let workspace_line = renderer
        .frames
        .last()
        .unwrap()
        .lines
        .iter()
        .find(|line| line.contains("Workspaces"))
        .expect("workspace row should render");
    assert!(workspace_line.contains("bad* invalid: session is empty"));
    assert!(!workspace_line.contains("Enter start/attach"));
}

#[test]
fn tmux_manager_panel_blocks_invalid_workspace_launch_before_runner_dispatch() {
    let snapshot = TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState::default(),
        current: None,
    };
    let mut panel = TmuxManagerPanelState::open_for_snapshot_with_workspaces(
        &snapshot,
        vec![gromaq::app::TmuxWorkspaceUiPreset::new(
            "bad",
            TmuxWorkspaceSettings::default(),
        )],
    );
    panel.focus_next();
    panel.focus_next();
    panel.focus_next();
    assert_eq!(panel.focus(), TmuxManagerFocus::Workspaces);

    assert_eq!(
        panel.handle_key(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::Consumed
    );
    assert_eq!(
        panel.workspace_feedback(),
        Some("workspace bad invalid: session is empty")
    );
}
