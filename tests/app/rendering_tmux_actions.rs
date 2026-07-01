use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerPanelState};
use gromaq::tmux::{
    TmuxManagerCurrent, TmuxManagerSnapshot, TmuxManagerStatus, TmuxPane, TmuxSession, TmuxState,
    TmuxWindow,
};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::support::{MockFrameRenderer, MockPtySession};

#[test]
fn native_terminal_runtime_renders_selected_action_command_and_key_hint() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 180,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    panel.focus_next();
    panel.focus_next();
    panel.focus_next();
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &panel)
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    let action_line = &frame.lines[6];
    assert!(action_line.contains("Enter split-pane-right"));
    assert!(action_line.contains("tmux split-window -h"));
    assert!(action_line.contains("Ctrl-b %"));
}

#[test]
fn native_terminal_runtime_renders_action_choices_with_shortcuts() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 320,
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

    let action_line = &renderer.frames.last().unwrap().lines[6];
    assert!(action_line.contains("s split-pane-right"), "{action_line}");
    assert!(action_line.contains("v split-pane-down"), "{action_line}");
    assert!(action_line.contains("c new-window"), "{action_line}");
    assert!(action_line.contains("q kill-session"), "{action_line}");
    assert!(action_line.contains("? show-help"), "{action_line}");
}

#[test]
fn native_terminal_runtime_renders_help_catalog_after_help_shortcut() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 900,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    panel.handle_key(
        &Key::Character("?".into()),
        ModifiersState::empty(),
        &snapshot,
    );
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &panel)
            .unwrap()
    );

    let help_line = &renderer.frames.last().unwrap().lines[7];
    assert!(help_line.contains("tmux help"));
    assert!(help_line.contains("s split-pane-right tmux split-window -h Ctrl-b %"));
    assert!(help_line.contains("c new-window tmux new-window Ctrl-b c"));
    assert!(help_line.contains("q kill-session tmux kill-session -t <session>"));
    assert!(help_line.contains("? show-help tmux list-keys Ctrl-b ?"));
    assert!(help_line.contains("r refresh tmux refresh snapshot no tmux key"));
}

#[test]
fn native_terminal_runtime_marks_unavailable_actions_without_active_tmux_target() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 900,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let snapshot = TmuxManagerSnapshot {
        status: TmuxManagerStatus::NoServer,
        state: TmuxState::default(),
        current: None,
    };
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    panel.focus_next();
    panel.focus_next();
    panel.focus_next();
    for _ in 0..3 {
        panel.handle_key(
            &Key::Named(NamedKey::ArrowUp),
            ModifiersState::empty(),
            &snapshot,
        );
    }
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &panel)
            .unwrap()
    );

    let action_line = &renderer.frames.last().unwrap().lines[6];
    assert!(
        action_line.contains("Enter split-pane-right[needs-active]"),
        "{action_line}"
    );
    assert!(action_line.contains("s split-pane-right[needs-active]*"));
    assert!(action_line.contains("v split-pane-down[needs-active]"));
    assert!(action_line.contains("z zoom-pane[needs-active]"));
    assert!(action_line.contains("t start-session"));
    assert!(!action_line.contains("t start-session[needs-active]"));
    assert!(action_line.contains("? show-help"));
    assert!(!action_line.contains("? show-help[needs-active]"));
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
