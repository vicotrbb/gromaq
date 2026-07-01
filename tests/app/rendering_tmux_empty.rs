use gromaq::app::{
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerPanelState, TmuxStatusKind,
    TmuxUiSnapshot,
};
use gromaq::tmux::{
    TmuxManagerSnapshot, TmuxManagerStatus, TmuxPane, TmuxSession, TmuxState, TmuxWindow,
};

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

#[test]
fn detached_status_snapshot_surfaces_available_window_and_active_pane() {
    let snapshot = TmuxUiSnapshot::from_manager_snapshot(&detached_manager_snapshot_with_state());

    assert_eq!(snapshot.status, TmuxStatusKind::Detached);
    assert_eq!(snapshot.current_session.as_deref(), Some("alpha"));
    assert_eq!(snapshot.current_window.as_deref(), Some("1:code"));
    assert_eq!(snapshot.visible_windows, vec!["0:shell", "1:code*"]);
    assert_eq!(snapshot.pane_count, Some(2));
    assert_eq!(snapshot.active_pane_id.as_deref(), Some("%2"));
    assert_eq!(snapshot.active_pane_command.as_deref(), Some("nvim"));
}

#[test]
fn detached_status_snapshot_falls_back_to_first_window_and_pane_without_active_flags() {
    let snapshot =
        TmuxUiSnapshot::from_manager_snapshot(&detached_manager_snapshot_without_active_flags());

    assert_eq!(snapshot.status, TmuxStatusKind::Detached);
    assert_eq!(snapshot.current_session.as_deref(), Some("alpha"));
    assert_eq!(snapshot.current_window.as_deref(), Some("0:shell"));
    assert_eq!(snapshot.pane_count, Some(1));
    assert_eq!(snapshot.active_pane_id.as_deref(), Some("%1"));
    assert_eq!(snapshot.active_pane_command.as_deref(), Some("zsh"));
}

#[test]
fn attached_status_snapshot_keeps_current_window_index_when_state_is_incomplete() {
    let mut manager = detached_manager_snapshot_with_state();
    manager.current = Some(gromaq::tmux::TmuxManagerCurrent {
        session_name: "alpha".to_owned(),
        window_index: 7,
        pane_id: "%7".to_owned(),
    });

    let snapshot = TmuxUiSnapshot::from_manager_snapshot(&manager);

    assert_eq!(snapshot.status, TmuxStatusKind::Attached);
    assert_eq!(snapshot.current_window.as_deref(), Some("7"));
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
            sessions: vec![TmuxSession {
                name: "alpha".to_owned(),
                attached: false,
            }],
            windows: Vec::new(),
            panes: Vec::new(),
        },
        current: None,
    }
}

fn detached_manager_snapshot_with_state() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState {
            sessions: vec![TmuxSession {
                name: "alpha".to_owned(),
                attached: false,
            }],
            windows: vec![
                TmuxWindow {
                    session_name: "alpha".to_owned(),
                    index: 0,
                    name: "shell".to_owned(),
                    active: false,
                },
                TmuxWindow {
                    session_name: "alpha".to_owned(),
                    index: 1,
                    name: "code".to_owned(),
                    active: true,
                },
            ],
            panes: vec![
                TmuxPane {
                    session_name: "alpha".to_owned(),
                    window_index: 1,
                    index: 0,
                    id: "%1".to_owned(),
                    title: "shell".to_owned(),
                    current_command: "zsh".to_owned(),
                    active: false,
                    width: Some(100),
                    height: Some(30),
                },
                TmuxPane {
                    session_name: "alpha".to_owned(),
                    window_index: 1,
                    index: 1,
                    id: "%2".to_owned(),
                    title: "editor".to_owned(),
                    current_command: "nvim".to_owned(),
                    active: true,
                    width: Some(100),
                    height: Some(30),
                },
            ],
        },
        current: None,
    }
}

fn detached_manager_snapshot_without_active_flags() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState {
            sessions: vec![TmuxSession {
                name: "alpha".to_owned(),
                attached: false,
            }],
            windows: vec![TmuxWindow {
                session_name: "alpha".to_owned(),
                index: 0,
                name: "shell".to_owned(),
                active: false,
            }],
            panes: vec![TmuxPane {
                session_name: "alpha".to_owned(),
                window_index: 0,
                index: 0,
                id: "%1".to_owned(),
                title: "shell".to_owned(),
                current_command: "zsh".to_owned(),
                active: false,
                width: Some(100),
                height: Some(30),
            }],
        },
        current: None,
    }
}
