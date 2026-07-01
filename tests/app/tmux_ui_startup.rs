use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use gromaq::tmux::{TmuxManagerSnapshot, TmuxManagerStatus, TmuxState};

use crate::support::{MockFrameRenderer, MockPtySession};

#[test]
fn runtime_open_keeps_tmux_manager_panel_visible_when_already_open() {
    let snapshot = TmuxManagerSnapshot {
        status: TmuxManagerStatus::NoServer,
        state: TmuxState::default(),
        current: None,
    };
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();

    runtime.open_tmux_manager_panel_with_workspaces(snapshot.clone(), Vec::new());
    runtime.open_tmux_manager_panel_with_workspaces(snapshot, Vec::new());

    assert!(runtime.tmux_manager_panel_is_open());
    let mut renderer = MockFrameRenderer::default();
    assert!(
        runtime
            .render_terminal_frame_with_status_overlay(&mut renderer, Some("144 fps"))
            .unwrap()
    );
    assert!(renderer.frames.last().unwrap().lines[2].contains("tmux manager"));
}
