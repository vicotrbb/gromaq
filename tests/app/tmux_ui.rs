use gromaq::app::{
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerFocus, TmuxManagerKeyOutcome,
    TmuxManagerPanelState,
};
use gromaq::tmux::{
    ActionId, TmuxManagerCurrent, TmuxManagerSnapshot, TmuxPane, TmuxSession, TmuxState, TmuxWindow,
};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::support::{MockFrameRenderer, MockPtySession};

#[test]
fn tmux_manager_panel_handles_navigation_actions_confirmation_and_close() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);

    assert_eq!(
        panel.handle_key(
            &Key::Named(NamedKey::ArrowDown),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::Consumed
    );
    assert_eq!(panel.selected_session_name(&snapshot), Some("beta"));

    assert_eq!(
        panel.handle_key(
            &Key::Character("k".into()),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::Consumed
    );
    assert_eq!(panel.selected_session_name(&snapshot), Some("alpha"));

    assert_eq!(
        panel.handle_key(
            &Key::Named(NamedKey::ArrowRight),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::Consumed
    );
    assert_eq!(panel.focus(), TmuxManagerFocus::Windows);

    panel.handle_key(
        &Key::Named(NamedKey::ArrowRight),
        ModifiersState::empty(),
        &snapshot,
    );
    panel.handle_key(
        &Key::Named(NamedKey::ArrowRight),
        ModifiersState::empty(),
        &snapshot,
    );
    assert_eq!(panel.focus(), TmuxManagerFocus::Actions);

    assert_eq!(
        panel.handle_key(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::ActionRequested(ActionId::SplitPaneRight)
    );
    assert_eq!(panel.pending_action(), Some("split-pane-right"));

    panel.handle_key(
        &Key::Named(NamedKey::ArrowDown),
        ModifiersState::empty(),
        &snapshot,
    );
    assert_eq!(
        panel.handle_key(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::ConfirmationRequired(ActionId::KillWindow)
    );
    assert_eq!(
        panel.confirmation_message(),
        Some("confirm kill-window with y")
    );
    assert_eq!(
        panel.handle_key(
            &Key::Character("y".into()),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::ConfirmedAction(ActionId::KillWindow)
    );
    assert_eq!(panel.confirmation_message(), None);

    assert_eq!(
        panel.handle_key(
            &Key::Named(NamedKey::Escape),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::Close
    );
    assert!(!panel.is_open());
}

#[test]
fn closed_tmux_manager_panel_does_not_consume_shell_input() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    panel.close();

    assert_eq!(
        panel.handle_key(
            &Key::Character("a".into()),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::Ignored
    );
}

#[test]
fn runtime_toggle_opens_and_closes_renderable_tmux_manager_panel() {
    let snapshot = manager_snapshot();
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let mut renderer = MockFrameRenderer::default();

    assert!(!runtime.tmux_manager_panel_is_open());
    runtime.toggle_tmux_manager_panel(snapshot.clone());
    assert!(runtime.tmux_manager_panel_is_open());
    assert!(
        runtime
            .render_terminal_frame_with_status_overlay(&mut renderer, Some("144 fps"))
            .unwrap()
    );
    assert!(renderer.frames.last().unwrap().lines[2].contains("tmux manager"));

    runtime.toggle_tmux_manager_panel(snapshot);
    assert!(!runtime.tmux_manager_panel_is_open());
}

#[test]
fn runtime_refreshes_open_tmux_manager_panel_snapshot() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    runtime.toggle_tmux_manager_panel(manager_snapshot());

    runtime.refresh_tmux_manager_panel(refreshed_manager_snapshot());

    assert!(runtime.tmux_manager_panel_is_open());
    let mut renderer = MockFrameRenderer::default();
    assert!(
        runtime
            .render_terminal_frame_with_status_overlay(&mut renderer, Some("144 fps"))
            .unwrap()
    );
    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[3].contains("Sessions gamma*"));
    assert!(!frame.lines[3].contains("alpha"));
}

fn manager_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
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

fn refreshed_manager_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        state: TmuxState {
            sessions: vec![TmuxSession {
                name: "gamma".to_owned(),
                attached: true,
            }],
            windows: vec![TmuxWindow {
                session_name: "gamma".to_owned(),
                index: 0,
                name: "ops".to_owned(),
                active: true,
            }],
            panes: vec![TmuxPane {
                session_name: "gamma".to_owned(),
                window_index: 0,
                index: 0,
                id: "%9".to_owned(),
                title: "monitor".to_owned(),
                current_command: "htop".to_owned(),
                active: true,
                width: Some(80),
                height: Some(24),
            }],
        },
        current: Some(TmuxManagerCurrent {
            session_name: "gamma".to_owned(),
            window_index: 0,
            pane_id: "%9".to_owned(),
        }),
    }
}
