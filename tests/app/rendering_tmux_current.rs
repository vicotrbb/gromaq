use crate::support::{MockFrameRenderer, MockPtySession};
use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerPanelState};
use gromaq::tmux::{
    TmuxManagerCurrent, TmuxManagerSnapshot, TmuxManagerStatus, TmuxPane, TmuxSession, TmuxState,
    TmuxWindow,
};
use winit::keyboard::{Key, ModifiersState, NamedKey};

#[test]
fn native_terminal_runtime_marks_current_session_and_window_after_selection_moves() {
    let snapshot = manager_snapshot();
    let mut renderer = MockFrameRenderer::default();

    let mut session_runtime = runtime();
    let mut session_panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    session_panel.handle_key(
        &Key::Named(NamedKey::ArrowDown),
        ModifiersState::empty(),
        &snapshot,
    );
    assert!(
        session_runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &session_panel)
            .unwrap()
    );
    let frame = renderer.frames.last().unwrap();
    assert!(
        frame.lines[3].contains("Sessions alpha@ beta*"),
        "{frame:?}"
    );

    let mut window_runtime = runtime();
    let mut window_panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    window_panel.focus_next();
    window_panel.handle_key(
        &Key::Named(NamedKey::ArrowUp),
        ModifiersState::empty(),
        &snapshot,
    );
    assert!(
        window_runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &window_panel)
            .unwrap()
    );
    let frame = renderer.frames.last().unwrap();
    assert!(
        frame.lines[4].contains("Windows 0:shell* 1:code@"),
        "{frame:?}"
    );
}

fn runtime() -> NativeTerminalRuntime<MockPtySession> {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 120,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    runtime
}

fn manager_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState {
            sessions: vec![
                TmuxSession {
                    name: "alpha".to_owned(),
                    attached: true,
                },
                TmuxSession {
                    name: "beta".to_owned(),
                    attached: false,
                },
            ],
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
