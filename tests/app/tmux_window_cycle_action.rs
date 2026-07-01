use std::cell::RefCell;

use gromaq::app::TmuxManagerPanelState;
use gromaq::tmux::{
    ActionId, TmuxActionResult, TmuxCommandOutput, TmuxCommandRunner, TmuxError,
    TmuxManagerCurrent, TmuxManagerSnapshot, TmuxManagerStatus, TmuxPane, TmuxSession, TmuxState,
    TmuxWindow,
};
use winit::keyboard::{Key, ModifiersState};

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

    fn remaining_calls(&self) -> usize {
        self.calls.borrow().len()
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
fn tmux_manager_panel_dispatches_next_window_to_selected_window() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    let runner = FakeRunner::new(vec![vec!["next-window", "-t", "alpha:1"]]);

    let outcome = panel.handle_key(
        &Key::Character("n".into()),
        ModifiersState::empty(),
        &snapshot,
    );
    let result = panel
        .dispatch_action_outcome(outcome, &snapshot, &runner)
        .unwrap();

    assert!(matches!(
        result,
        TmuxActionResult::Success {
            action_id: ActionId::NextWindow,
            ..
        }
    ));
    assert_eq!(runner.remaining_calls(), 0);
    assert_eq!(panel.last_action_feedback(), Some("next-window success"));
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
