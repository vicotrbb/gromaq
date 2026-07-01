use gromaq::app::{
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerPanelState, TmuxStatusKind,
    TmuxUiSnapshot,
};
use gromaq::tmux::TmuxManagerSnapshot;

use crate::support::{MockFrameRenderer, MockPtySession};

#[test]
fn native_terminal_runtime_hides_cursor_when_tmux_manager_covers_it() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime
        .write_startup_text("ready\r\nprompt\r\ncovered > ")
        .unwrap();
    let snapshot = TmuxManagerSnapshot::no_server();
    let panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &panel)
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines.iter().any(|line| line.contains("tmux manager")));
    assert!(!frame.cursor.visible);
    assert!(runtime.terminal().dump_cursor().visible);
}

#[test]
fn native_terminal_runtime_hides_cursor_when_tmux_status_strip_covers_it() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 4,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime
        .write_startup_text("ready\r\nlogs\r\nmore\r\ncovered > ")
        .unwrap();
    let snapshot = TmuxUiSnapshot {
        status: TmuxStatusKind::NoServer,
        current_session: None,
        current_window: None,
        visible_windows: Vec::new(),
        pane_count: None,
        active_pane_id: None,
        active_pane_command: None,
        pending_feedback: None,
        confirmation_feedback: None,
    };
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_status_strip(&mut renderer, &snapshot)
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[3].contains("tmux: no server"));
    assert!(!frame.cursor.visible);
    assert!(runtime.terminal().dump_cursor().visible);
}
