use gromaq::app::TmuxUiSnapshot;
use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxStatusKind};
use gromaq::tmux::{
    TmuxManagerCurrent, TmuxManagerSnapshot, TmuxManagerStatus, TmuxPane, TmuxSession, TmuxState,
    TmuxWindow,
};

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

#[test]
fn retained_tmux_status_strip_renders_when_terminal_frame_is_clean() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 72,
        terminal_rows: 5,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let mut renderer = MockFrameRenderer::default();

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());
    runtime.set_tmux_status_snapshot(attached_snapshot());

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[4].contains("tmux: attached"));
    assert!(frame.lines[4].contains("alpha"));
}

#[test]
fn retained_tmux_status_strip_renders_when_viewport_has_no_blank_row() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 96,
        terminal_rows: 3,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime
        .write_startup_text(
            "build finished\r\n................................ rb 2.7.5 15:42\r\n> ",
        )
        .unwrap();
    runtime.set_tmux_status_snapshot(attached_snapshot());
    let mut renderer = MockFrameRenderer::default();

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[2].contains("tmux: attached"));
    assert!(frame.lines[2].contains("manager Cmd/Ctrl+Shift+T"));
    assert_eq!(
        runtime.terminal().dump_grid().line_text(2),
        ">",
        "status strip must stay frame-only and not mutate shell grid"
    );
}

#[test]
fn hidden_tmux_status_strip_still_allows_retained_manager_panel() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 96,
        terminal_rows: 9,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    runtime.set_tmux_status_strip_enabled(false);
    runtime.set_tmux_status_snapshot(attached_snapshot());
    let snapshot = manager_snapshot();
    runtime.toggle_tmux_manager_panel(snapshot);
    let mut renderer = MockFrameRenderer::default();

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());

    let frame = renderer.frames.last().unwrap();
    assert!(!runtime.last_rendered_tmux_status_strip());
    assert!(runtime.last_rendered_tmux_manager_panel());
    assert!(frame.lines.iter().any(|line| line.contains("tmux manager")));
    assert!(
        !frame
            .lines
            .iter()
            .any(|line| line.contains("tmux: attached"))
    );
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

fn manager_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState {
            sessions: vec![TmuxSession {
                name: "alpha".to_owned(),
                attached: true,
            }],
            windows: vec![TmuxWindow {
                session_name: "alpha".to_owned(),
                index: 1,
                name: "code".to_owned(),
                active: true,
            }],
            panes: vec![TmuxPane {
                session_name: "alpha".to_owned(),
                window_index: 1,
                index: 0,
                id: "%2".to_owned(),
                title: "editor".to_owned(),
                current_command: "nvim".to_owned(),
                active: true,
                width: Some(100),
                height: Some(30),
            }],
        },
        current: Some(TmuxManagerCurrent {
            session_name: "alpha".to_owned(),
            window_index: 1,
            pane_id: "%2".to_owned(),
        }),
    }
}
