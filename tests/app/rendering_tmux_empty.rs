use gromaq::app::{
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerPanelState, TmuxStatusKind,
    TmuxUiSnapshot,
};
use gromaq::tmux::{TmuxManagerSnapshot, TmuxManagerStatus, TmuxState};

use crate::support::{MockFrameRenderer, MockPtySession};

#[test]
fn native_terminal_runtime_renders_empty_tmux_manager_start_session_affordance() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 96,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let snapshot = empty_manager_snapshot();
    let panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &panel)
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[3].contains("Sessions none"));
    assert!(frame.lines[4].contains("Windows none"));
    assert!(frame.lines[5].contains("Panes none"));
    assert!(frame.lines[6].contains("Enter start-session"));
    assert!(frame.lines[7].contains("tmux new-session -d -s <session>"));
    assert!(frame.lines[7].contains("Enter start-session to create"));
}

#[test]
fn native_terminal_runtime_renders_outside_tmux_attach_command_hint() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 128,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let snapshot = detached_manager_snapshot();
    let panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &panel)
            .unwrap()
    );

    let hint_line = &renderer.frames.last().unwrap().lines[7];
    assert!(hint_line.contains("Outside tmux"), "{hint_line}");
    assert!(
        hint_line.contains("tmux attach-session -t <session>"),
        "{hint_line}"
    );
}

#[test]
fn native_terminal_runtime_renders_missing_tmux_manager_explanation() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 96,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let snapshot = missing_manager_snapshot();
    let panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &panel)
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[7].contains("tmux missing"));
    assert!(!frame.lines[7].contains("No tmux server"));
}

#[test]
fn status_snapshot_distinguishes_missing_tmux_from_no_server() {
    let snapshot = TmuxUiSnapshot::from_manager_snapshot(&missing_manager_snapshot());

    assert_eq!(snapshot.status, TmuxStatusKind::Missing);
}

fn empty_manager_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::NoServer,
        state: TmuxState {
            sessions: Vec::new(),
            windows: Vec::new(),
            panes: Vec::new(),
        },
        current: None,
    }
}

fn missing_manager_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::Missing,
        state: TmuxState {
            sessions: Vec::new(),
            windows: Vec::new(),
            panes: Vec::new(),
        },
        current: None,
    }
}

fn detached_manager_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState {
            sessions: vec![gromaq::tmux::TmuxSession {
                name: "alpha".to_owned(),
                attached: false,
            }],
            windows: Vec::new(),
            panes: Vec::new(),
        },
        current: None,
    }
}
