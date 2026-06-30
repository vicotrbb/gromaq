use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use gromaq::app::{TmuxStatusKind, TmuxUiSnapshot};

use crate::support::{MockFrameRenderer, MockPtySession};

#[test]
fn native_terminal_runtime_renders_tmux_assist_overlay_once() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 48,
        terminal_rows: 4,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    runtime.show_tmux_assist_overlay();
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_status_overlay(&mut renderer, Some("144 fps"))
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[0].contains("tmux split-window -h | Ctrl-b %"));
    assert!(!frame.lines[0].contains("144 fps"));
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "ready");
    assert_eq!(runtime.terminal().dump_grid().line_text(1), ">");

    runtime.invalidate_terminal_frame();
    assert!(
        runtime
            .render_terminal_frame_with_status_overlay(&mut renderer, Some("144 fps"))
            .unwrap()
    );
    assert!(renderer.frames.last().unwrap().lines[0].contains("144 fps"));
}

#[test]
fn native_terminal_runtime_renders_tmux_assist_overlay_below_right_prompt() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 64,
        terminal_rows: 5,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime
        .write_startup_text("ready\r\n................................ rb 2.7.5 15:42\r\n> ")
        .unwrap();
    runtime.show_tmux_assist_overlay();
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_status_overlay(&mut renderer, Some("144 fps"))
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[2].contains("tmux split-window -h | Ctrl-b %"));
    assert!(frame.lines[1].contains("rb 2.7.5 15:42"));
    assert_eq!(
        runtime.terminal().dump_grid().line_text(1),
        "................................ rb 2.7.5 15:42"
    );
}

#[test]
fn native_terminal_runtime_renders_persistent_tmux_status_strip_without_mutating_grid() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 72,
        terminal_rows: 5,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let snapshot = attached_snapshot();
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_status_strip(&mut renderer, &snapshot)
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[4].contains("tmux: attached"));
    assert!(frame.lines[4].contains("alpha"));
    assert!(frame.lines[4].contains("1:code"));
    assert!(frame.lines[4].contains("panes 3"));
    assert!(frame.lines[4].contains("%2 nvim"));
    assert!(frame.lines[4].contains("split right ok"));
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "ready");
    assert_eq!(runtime.terminal().dump_grid().line_text(1), ">");
    assert!(frame.dirty_regions.iter().any(|region| {
        region.row == 4 && region.col == 0 && region.rows == 1 && region.cols == 72
    }));
}

#[test]
fn native_terminal_runtime_renders_tmux_no_server_status_strip() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 40,
        terminal_rows: 3,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("> ").unwrap();
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

    assert!(renderer.frames.last().unwrap().lines[2].contains("tmux: no server"));
}

#[test]
fn native_terminal_runtime_truncates_tmux_status_strip_on_narrow_width() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 24,
        terminal_rows: 3,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("> ").unwrap();
    let mut snapshot = attached_snapshot();
    snapshot.current_window = Some("1:very-long-code-window".to_owned());
    snapshot.visible_windows = vec!["0:shell".to_owned(), "1:very-long-code-window*".to_owned()];
    snapshot.pane_count = Some(12);
    snapshot.active_pane_id = Some("%22".to_owned());
    snapshot.active_pane_command = Some("long-running-editor".to_owned());
    snapshot.pending_feedback = None;
    snapshot.confirmation_feedback = Some("confirm kill-window with y".to_owned());
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_status_strip(&mut renderer, &snapshot)
            .unwrap()
    );

    let strip = &renderer.frames.last().unwrap().lines[2];
    assert_eq!(strip.chars().count(), 24);
    assert!(strip.starts_with("tmux: attached"));
    assert!(strip.ends_with("..."));
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
        pending_feedback: Some("split right ok".to_owned()),
        confirmation_feedback: None,
    }
}
