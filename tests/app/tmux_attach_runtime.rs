use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerKeyOutcome};
use gromaq::tmux::{
    ActionId, TmuxActionResult, TmuxManagerCurrent, TmuxManagerSnapshot, TmuxManagerStatus,
    TmuxPane, TmuxSession, TmuxState, TmuxWindow,
};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::support::{MockFrameRenderer, MockPtySession};

#[test]
fn runtime_attach_session_action_reports_skipped_without_shell() {
    let mut snapshot = manager_snapshot();
    snapshot.current = None;
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 160,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    runtime.toggle_tmux_manager_panel(snapshot);

    let result = runtime
        .dispatch_tmux_manager_terminal_action(TmuxManagerKeyOutcome::ActionRequested(
            ActionId::AttachSession,
        ))
        .unwrap();

    assert!(matches!(
        result,
        TmuxActionResult::Skipped {
            action_id: ActionId::AttachSession,
            ..
        }
    ));
    runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Escape), ModifiersState::empty());
    let mut renderer = MockFrameRenderer::default();

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());

    let frame = renderer.frames.last().unwrap();
    assert!(
        frame.lines[7].contains("attach-session skipped: shell not started"),
        "{:?}",
        frame.lines[7]
    );
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
