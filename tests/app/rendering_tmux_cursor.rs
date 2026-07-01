use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerPanelState};
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
