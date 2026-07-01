use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use gromaq::app::{TmuxManagerFocus, TmuxManagerPanelState, TmuxStatusKind, TmuxUiSnapshot};
use gromaq::tmux::{
    TmuxManagerCurrent, TmuxManagerSnapshot, TmuxManagerStatus, TmuxPane, TmuxSession, TmuxState,
    TmuxWindow,
};

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
        terminal_cols: 120,
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
    assert!(frame.lines[4].contains("manager Cmd/Ctrl+Shift+T"));
    assert!(frame.lines[4].contains("alpha"));
    assert!(frame.lines[4].contains("1:code"));
    assert!(frame.lines[4].contains("panes 3"));
    assert!(frame.lines[4].contains("%2 nvim"));
    assert!(frame.lines[4].contains("split right ok"));
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "ready");
    assert_eq!(runtime.terminal().dump_grid().line_text(1), ">");
    assert!(frame.dirty_regions.iter().any(|region| {
        region.row == 4 && region.col == 0 && region.rows == 1 && region.cols == 120
    }));
}

#[test]
fn native_terminal_runtime_renders_tmux_no_server_status_strip() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 64,
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

    let strip = &renderer.frames.last().unwrap().lines[2];
    assert!(strip.contains("tmux: no server"));
    assert!(strip.contains("manager Cmd/Ctrl+Shift+T"));
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
    snapshot.confirmation_feedback = Some("confirm kill-window with y, n/Esc cancels".to_owned());
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

#[test]
fn tmux_manager_panel_state_opens_on_current_target_and_tracks_confirmation() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);

    assert!(panel.is_open());
    assert_eq!(panel.focus(), TmuxManagerFocus::Sessions);
    assert_eq!(panel.selected_session_name(&snapshot), Some("alpha"));
    assert_eq!(
        panel.selected_window_label(&snapshot),
        Some("1:code".to_owned())
    );
    assert_eq!(panel.selected_pane_id(&snapshot), Some("%2"));

    panel.focus_next();
    assert_eq!(panel.focus(), TmuxManagerFocus::Windows);
    panel.request_action("kill-window", true);
    assert_eq!(
        panel.confirmation_message(),
        Some("confirm kill-window with y, n/Esc cancels")
    );
    panel.cancel_confirmation();
    assert_eq!(panel.confirmation_message(), None);

    panel.request_action("split-pane-right", false);
    assert_eq!(panel.pending_action(), Some("split-pane-right"));
    panel.close();
    assert!(!panel.is_open());
}

#[test]
fn native_terminal_runtime_renders_compact_tmux_manager_panel() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    panel.focus_next();
    panel.request_action("kill-window", true);
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &panel)
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[2].contains("tmux manager"));
    assert!(frame.lines[3].contains("Sessions alpha* beta"));
    assert!(frame.lines[4].contains("Windows 0:shell 1:code*"));
    assert!(frame.lines[5].contains("Panes %1 shell:zsh 100x30 %2 editor:nvim* 100x30"));
    assert!(frame.lines[6].contains("Enter split-pane-right"));
    assert!(frame.lines[6].contains("Ctrl-b %"));
    assert!(frame.lines[7].contains("confirm kill-window with y, n/Esc cancels"));
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "ready");
    assert_eq!(runtime.terminal().dump_grid().line_text(1), ">");
}

#[test]
fn native_terminal_runtime_renders_tmux_manager_executable_actions() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 320,
        terminal_rows: 9,
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
    let action_line = &frame.lines[6];
    let shortcut_line = &frame.lines[7];
    assert!(shortcut_line.contains("Shortcuts"));
    assert!(shortcut_line.contains("c new-window"));
    assert!(shortcut_line.contains("w kill-window"));
    for action in [
        "attach-session",
        "start-session",
        "detach-session",
        "split-pane-right",
        "split-pane-down",
        "new-window",
        "rename-session",
        "rename-window",
        "next-window",
        "previous-window",
        "zoom-pane",
        "select-pane",
        "kill-pane",
        "kill-window",
        "kill-session",
        "show-help",
    ] {
        assert!(action_line.contains(action), "{action_line}");
    }
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
