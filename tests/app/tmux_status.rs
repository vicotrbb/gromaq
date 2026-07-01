use gromaq::app::TmuxUiSnapshot;
use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxStatusKind};

use crate::support::{MockFrameRenderer, MockPtySession};

#[test]
fn native_terminal_runtime_renders_retained_tmux_status_strip_on_normal_frame() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 72,
        terminal_rows: 5,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    runtime.set_tmux_status_snapshot(attached_snapshot());
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_status_overlay(&mut renderer, Some("144 fps"))
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[4].contains("tmux: attached"));
    assert!(frame.lines[4].contains("alpha"));
    assert!(frame.lines[4].contains("1:code"));
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "ready");
    assert_eq!(runtime.terminal().dump_grid().line_text(1), ">");
}

fn attached_snapshot() -> TmuxUiSnapshot {
    TmuxUiSnapshot {
        status: TmuxStatusKind::Attached,
        current_session: Some("alpha".to_owned()),
        current_window: Some("1:code".to_owned()),
        visible_windows: vec!["0:shell".to_owned(), "1:code*".to_owned()],
        pane_count: Some(3),
        active_pane_id: Some("%2".to_owned()),
        active_pane_command: Some("nvim".to_owned()),
        pending_feedback: None,
        confirmation_feedback: None,
    }
}
