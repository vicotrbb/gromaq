use std::cell::RefCell;

use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use gromaq::tmux::{
    ActionId, TmuxActionResult, TmuxCommandOutput, TmuxCommandRunner, TmuxError,
    TmuxManagerCurrent, TmuxManagerSnapshot, TmuxManagerStatus, TmuxPane, TmuxSession, TmuxState,
    TmuxWindow,
};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::support::{MockFrameRenderer, MockPtySession};

#[derive(Debug)]
struct FakeRunner {
    calls: RefCell<Vec<Vec<&'static str>>>,
}

impl FakeRunner {
    fn new(calls: Vec<Vec<&'static str>>) -> Self {
        Self {
            calls: RefCell::new(calls),
        }
    }
}

impl TmuxCommandRunner for FakeRunner {
    fn run_tmux(&self, args: &[&str]) -> Result<TmuxCommandOutput, TmuxError> {
        let expected = self.calls.borrow_mut().remove(0);
        assert_eq!(args, expected.as_slice());
        Ok(TmuxCommandOutput::new(String::new(), String::new()))
    }
}

#[test]
fn runtime_refresh_preserves_action_feedback_in_status_strip() {
    let snapshot = manager_snapshot();
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 120,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    runtime.toggle_tmux_manager_panel(snapshot.clone());
    let runner = FakeRunner::new(vec![vec!["split-window", "-h", "-t", "%2"]]);

    for _ in 0..3 {
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::ArrowRight), ModifiersState::empty());
    }
    let outcome =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty());
    let result = runtime
        .dispatch_tmux_manager_action(outcome, &runner)
        .unwrap();
    assert!(matches!(
        result,
        TmuxActionResult::Success {
            action_id: ActionId::SplitPaneRight,
            ..
        }
    ));

    runtime.refresh_tmux_manager_panel(snapshot);
    runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Escape), ModifiersState::empty());
    let mut renderer = MockFrameRenderer::default();

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[7].contains("tmux: attached"));
    assert!(frame.lines[7].contains("split-pane-right success"));
}

#[test]
fn runtime_shows_confirmation_feedback_in_status_strip() {
    let snapshot = manager_snapshot();
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 200,
        terminal_rows: 12,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    runtime.toggle_tmux_manager_panel(snapshot);

    for _ in 0..3 {
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::ArrowRight), ModifiersState::empty());
    }
    runtime.handle_tmux_manager_key(&Key::Named(NamedKey::ArrowDown), ModifiersState::empty());
    let outcome =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty());
    assert_eq!(
        outcome,
        gromaq::app::TmuxManagerKeyOutcome::ConfirmationRequired(ActionId::KillWindow)
    );
    let mut renderer = MockFrameRenderer::default();

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[11].contains("tmux: attached"));
    assert!(frame.lines[11].contains("confirm: confirm kill-window"));
    assert!(frame.lines[11].contains("Ctrl-b &"));
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
