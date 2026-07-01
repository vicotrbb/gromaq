use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerKeyOutcome};
use gromaq::tmux::{
    ActionId, TmuxManagerCurrent, TmuxManagerSnapshot, TmuxManagerStatus, TmuxPane, TmuxSession,
    TmuxState, TmuxWindow,
};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::support::MockPtySession;

#[test]
fn runtime_refresh_preserves_actions_focus_after_state_changes() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 120,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    runtime.toggle_tmux_manager_panel(manager_snapshot("alpha", 1, "%2", "nvim"));

    for _ in 0..3 {
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::ArrowRight), ModifiersState::empty());
    }
    assert_eq!(
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()),
        TmuxManagerKeyOutcome::ActionRequested(ActionId::SplitPaneRight)
    );

    runtime.refresh_tmux_manager_panel(manager_snapshot("alpha", 2, "%3", "zsh"));

    assert_eq!(
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()),
        TmuxManagerKeyOutcome::ActionRequested(ActionId::SplitPaneRight)
    );
}

fn manager_snapshot(
    session_name: &str,
    window_index: u16,
    pane_id: &str,
    command: &str,
) -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState {
            sessions: vec![TmuxSession {
                name: session_name.to_owned(),
                attached: true,
            }],
            windows: vec![TmuxWindow {
                session_name: session_name.to_owned(),
                index: window_index,
                name: "code".to_owned(),
                active: true,
            }],
            panes: vec![TmuxPane {
                session_name: session_name.to_owned(),
                window_index,
                index: 0,
                id: pane_id.to_owned(),
                title: "editor".to_owned(),
                current_command: command.to_owned(),
                active: true,
                width: Some(100),
                height: Some(30),
            }],
        },
        current: Some(TmuxManagerCurrent {
            session_name: session_name.to_owned(),
            window_index,
            pane_id: pane_id.to_owned(),
        }),
    }
}
