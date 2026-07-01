use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerPanelState};
use gromaq::tmux::{
    TmuxManagerCurrent, TmuxManagerSnapshot, TmuxManagerStatus, TmuxPane, TmuxSession, TmuxState,
    TmuxWindow,
};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::support::{MockFrameRenderer, MockPtySession};

#[test]
fn native_terminal_runtime_renders_pane_titles_commands_and_dimensions() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 112,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let snapshot = manager_snapshot();
    let panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &panel)
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    let pane_line = &frame.lines[5];
    assert!(pane_line.contains("%1 shell:zsh 100x30"));
    assert!(pane_line.contains("%2 editor:nvim* 100x30"));
}

#[test]
fn native_terminal_runtime_renders_current_target_pane_details_in_header() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 160,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let snapshot = manager_snapshot();
    let panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &panel)
            .unwrap()
    );

    let header = &renderer.frames.last().unwrap().lines[2];
    assert!(header.contains("target alpha:1:%2"), "{header}");
    assert!(header.contains("editor:nvim"), "{header}");
    assert!(header.contains("100x30"), "{header}");
}

#[test]
fn native_terminal_runtime_marks_current_pane_after_selection_moves() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 112,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    panel.focus_next();
    panel.focus_next();
    panel.handle_key(
        &Key::Named(NamedKey::ArrowUp),
        ModifiersState::empty(),
        &snapshot,
    );
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &panel)
            .unwrap()
    );

    let pane_line = &renderer.frames.last().unwrap().lines[5];
    assert!(pane_line.contains("%1 shell:zsh* 100x30"), "{pane_line}");
    assert!(pane_line.contains("%2 editor:nvim 100x30@"), "{pane_line}");
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
        current: Some(TmuxManagerCurrent {
            session_name: "alpha".to_owned(),
            window_index: 1,
            pane_id: "%2".to_owned(),
        }),
    }
}
