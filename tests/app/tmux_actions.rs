use std::cell::RefCell;

use gromaq::app::{
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerKeyOutcome,
    TmuxManagerPanelState,
};
use gromaq::tmux::{
    ActionId, TmuxActionResult, TmuxCommandFailure, TmuxCommandOutput, TmuxCommandRunner,
    TmuxError, TmuxManagerCurrent, TmuxManagerSnapshot, TmuxPane, TmuxSession, TmuxState,
    TmuxWindow,
};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::support::MockPtySpawner;

#[derive(Debug)]
struct FakeRunner {
    calls: RefCell<Vec<ExpectedCall>>,
}

#[derive(Debug)]
struct ExpectedCall {
    args: Vec<&'static str>,
    result: Result<TmuxCommandOutput, TmuxError>,
}

impl FakeRunner {
    fn new(calls: Vec<ExpectedCall>) -> Self {
        Self {
            calls: RefCell::new(calls),
        }
    }

    fn remaining_calls(&self) -> usize {
        self.calls.borrow().len()
    }
}

impl ExpectedCall {
    fn success(args: &[&'static str]) -> Self {
        Self {
            args: args.to_vec(),
            result: Ok(TmuxCommandOutput::new(String::new(), String::new())),
        }
    }

    fn failure(args: &[&'static str], stderr: &'static str) -> Self {
        Self {
            args: args.to_vec(),
            result: Err(TmuxError::Command(TmuxCommandFailure::new(
                args.iter().map(|arg| (*arg).to_owned()).collect(),
                1,
                stderr.to_owned(),
            ))),
        }
    }
}

impl TmuxCommandRunner for FakeRunner {
    fn run_tmux(&self, args: &[&str]) -> Result<TmuxCommandOutput, TmuxError> {
        let expected = self.calls.borrow_mut().remove(0);
        assert_eq!(args, expected.args.as_slice());
        expected.result
    }
}

#[test]
fn tmux_manager_panel_dispatches_safe_action_through_action_runner() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    panel.focus_next();
    panel.focus_next();
    panel.focus_next();
    let runner = FakeRunner::new(vec![ExpectedCall::success(&[
        "split-window",
        "-h",
        "-t",
        "%2",
    ])]);

    let outcome = panel.handle_key(
        &Key::Named(NamedKey::Enter),
        ModifiersState::empty(),
        &snapshot,
    );
    let result = panel
        .dispatch_action_outcome(outcome, &snapshot, &runner)
        .unwrap();

    assert!(matches!(
        result,
        TmuxActionResult::Success {
            action_id: ActionId::SplitPaneRight,
            ..
        }
    ));
    assert_eq!(runner.remaining_calls(), 0);
    assert_eq!(
        panel.last_action_feedback(),
        Some("split-pane-right success")
    );
}

#[test]
fn tmux_manager_panel_waits_for_confirmation_before_kill_action_dispatch() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    panel.focus_next();
    panel.focus_next();
    panel.focus_next();
    panel.handle_key(
        &Key::Named(NamedKey::ArrowDown),
        ModifiersState::empty(),
        &snapshot,
    );
    let runner = FakeRunner::new(vec![ExpectedCall::success(&[
        "kill-window",
        "-t",
        "alpha:1",
    ])]);

    let needs_confirmation = panel.handle_key(
        &Key::Named(NamedKey::Enter),
        ModifiersState::empty(),
        &snapshot,
    );
    assert_eq!(
        needs_confirmation,
        TmuxManagerKeyOutcome::ConfirmationRequired(ActionId::KillWindow)
    );
    assert!(
        panel
            .dispatch_action_outcome(needs_confirmation, &snapshot, &runner)
            .is_none()
    );
    assert_eq!(runner.remaining_calls(), 1);

    let confirmed = panel.handle_key(
        &Key::Character("y".into()),
        ModifiersState::empty(),
        &snapshot,
    );
    let result = panel
        .dispatch_action_outcome(confirmed, &snapshot, &runner)
        .unwrap();

    assert!(matches!(
        result,
        TmuxActionResult::Success {
            action_id: ActionId::KillWindow,
            ..
        }
    ));
    assert_eq!(runner.remaining_calls(), 0);
    assert_eq!(panel.last_action_feedback(), Some("kill-window success"));
}

#[test]
fn tmux_manager_panel_reports_action_runner_failure_feedback() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    panel.focus_next();
    panel.focus_next();
    panel.focus_next();
    let runner = FakeRunner::new(vec![ExpectedCall::failure(
        &["split-window", "-h", "-t", "%2"],
        "no current client",
    )]);

    let outcome = panel.handle_key(
        &Key::Named(NamedKey::Enter),
        ModifiersState::empty(),
        &snapshot,
    );
    let result = panel
        .dispatch_action_outcome(outcome, &snapshot, &runner)
        .unwrap();

    assert!(matches!(
        result,
        TmuxActionResult::CommandFailed {
            action_id: ActionId::SplitPaneRight,
            ..
        }
    ));
    assert_eq!(
        panel.last_action_feedback(),
        Some("split-pane-right failed: no current client")
    );
}

#[test]
fn runtime_dispatches_open_manager_action_outcome_through_runner() {
    let snapshot = manager_snapshot();
    let mut runtime = NativeTerminalRuntime::<crate::support::MockPtySession>::new(
        NativeTerminalRuntimeConfig::default(),
    )
    .unwrap();
    runtime.toggle_tmux_manager_panel(snapshot);
    let runner = FakeRunner::new(vec![ExpectedCall::success(&[
        "split-window",
        "-h",
        "-t",
        "%2",
    ])]);

    runtime.handle_tmux_manager_key(&Key::Named(NamedKey::ArrowRight), ModifiersState::empty());
    runtime.handle_tmux_manager_key(&Key::Named(NamedKey::ArrowRight), ModifiersState::empty());
    runtime.handle_tmux_manager_key(&Key::Named(NamedKey::ArrowRight), ModifiersState::empty());
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
    assert_eq!(runner.remaining_calls(), 0);
}

#[test]
fn runtime_dispatches_attach_session_action_through_terminal_pty() {
    let mut snapshot = manager_snapshot();
    snapshot.current = None;
    let mut runtime = NativeTerminalRuntime::<crate::support::MockPtySession>::new(
        NativeTerminalRuntimeConfig::default(),
    )
    .unwrap();
    runtime.start_shell(&MockPtySpawner::default()).unwrap();
    runtime.toggle_tmux_manager_panel(snapshot);

    let result = runtime
        .dispatch_tmux_manager_terminal_action(TmuxManagerKeyOutcome::ActionRequested(
            ActionId::AttachSession,
        ))
        .unwrap();

    assert!(matches!(
        result,
        TmuxActionResult::Success {
            action_id: ActionId::AttachSession,
            ..
        }
    ));
    let input = runtime.shell_session().unwrap().input.borrow();
    assert_eq!(
        input.last().map(Vec::as_slice),
        Some(b"tmux attach-session -t alpha\r".as_slice())
    );
}

fn manager_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
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
