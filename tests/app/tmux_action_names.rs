use std::cell::RefCell;

use gromaq::app::{TmuxManagerKeyOutcome, TmuxManagerPanelState};
use gromaq::tmux::{
    ActionId, TmuxActionResult, TmuxCommandOutput, TmuxCommandRunner, TmuxError,
    TmuxManagerCurrent, TmuxManagerSnapshot, TmuxPane, TmuxSession, TmuxState, TmuxWindow,
};
use winit::keyboard::{Key, ModifiersState, NamedKey};

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
}

impl TmuxCommandRunner for FakeRunner {
    fn run_tmux(&self, args: &[&str]) -> Result<TmuxCommandOutput, TmuxError> {
        let expected = self.calls.borrow_mut().remove(0);
        assert_eq!(args, expected.args.as_slice());
        expected.result
    }
}

#[test]
fn tmux_manager_panel_collects_name_and_starts_session() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    focus_action_index(&mut panel, &snapshot, 3);
    let runner = FakeRunner::new(vec![ExpectedCall::success(&[
        "new-session",
        "-d",
        "-s",
        "delta",
    ])]);

    submit_action_name(&mut panel, &snapshot, "delta");
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
            action_id: ActionId::StartSession,
            ..
        }
    ));
    assert_eq!(runner.remaining_calls(), 0);
}

#[test]
fn tmux_manager_panel_collects_name_and_renames_session() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    focus_action_index(&mut panel, &snapshot, 7);
    let runner = FakeRunner::new(vec![ExpectedCall::success(&[
        "rename-session",
        "-t",
        "alpha",
        "omega",
    ])]);

    submit_action_name(&mut panel, &snapshot, "omega");
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
            action_id: ActionId::RenameSession,
            ..
        }
    ));
    assert_eq!(runner.remaining_calls(), 0);
}

#[test]
fn tmux_manager_panel_collects_name_and_renames_window() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    focus_action_index(&mut panel, &snapshot, 8);
    let runner = FakeRunner::new(vec![ExpectedCall::success(&[
        "rename-window",
        "-t",
        "alpha:1",
        "work",
    ])]);

    submit_action_name(&mut panel, &snapshot, "work");
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
            action_id: ActionId::RenameWindow,
            ..
        }
    ));
    assert_eq!(runner.remaining_calls(), 0);
}

fn focus_action_index(
    panel: &mut TmuxManagerPanelState,
    snapshot: &TmuxManagerSnapshot,
    index: usize,
) {
    panel.focus_next();
    panel.focus_next();
    panel.focus_next();
    for _ in 0..index {
        panel.handle_key(
            &Key::Named(NamedKey::ArrowDown),
            ModifiersState::empty(),
            snapshot,
        );
    }
}

fn submit_action_name(
    panel: &mut TmuxManagerPanelState,
    snapshot: &TmuxManagerSnapshot,
    value: &str,
) {
    assert_eq!(
        panel.handle_key(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty(),
            snapshot
        ),
        TmuxManagerKeyOutcome::Consumed
    );
    for character in value.chars() {
        assert_eq!(
            panel.handle_key(
                &Key::Character(character.to_string().into()),
                ModifiersState::empty(),
                snapshot
            ),
            TmuxManagerKeyOutcome::Consumed
        );
    }
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
