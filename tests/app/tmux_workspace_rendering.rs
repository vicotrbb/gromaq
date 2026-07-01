use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerPanelState};
use gromaq::config::TmuxWorkspaceSettings;
use gromaq::tmux::{TmuxManagerSnapshot, TmuxManagerStatus, TmuxState};

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
