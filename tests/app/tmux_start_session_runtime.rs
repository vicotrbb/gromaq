use std::cell::RefCell;

use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerKeyOutcome};
use gromaq::tmux::{
    ActionId, TmuxActionResult, TmuxCommandOutput, TmuxCommandRunner, TmuxError,
    TmuxManagerSnapshot, TmuxManagerStatus, TmuxState,
};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::support::{MockFrameRenderer, MockPtySession, MockPtySpawner};

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
fn runtime_start_session_action_attaches_created_session_through_pty() {
    let snapshot = TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState::default(),
        current: None,
    };
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 88,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.start_shell(&MockPtySpawner::default()).unwrap();
    runtime.toggle_tmux_manager_panel(snapshot);
    let runner = FakeRunner::new(vec![ExpectedCall::success(&[
        "new-session",
        "-d",
        "-s",
        "delta",
    ])]);

    assert_eq!(
        runtime.handle_tmux_manager_key(&Key::Character("t".into()), ModifiersState::empty()),
        TmuxManagerKeyOutcome::Consumed
    );
    for character in "delta".chars() {
        assert_eq!(
            runtime.handle_tmux_manager_key(
                &Key::Character(character.to_string().into()),
                ModifiersState::empty()
            ),
            TmuxManagerKeyOutcome::Consumed
        );
    }
    let outcome =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty());
    let result = runtime
        .dispatch_tmux_manager_action(outcome, &runner)
        .unwrap();

    assert!(matches!(
        result,
        TmuxActionResult::Success {
            action_id: ActionId::StartSession,
            ..
        }
    ));
    assert_eq!(runner.remaining_calls(), 0);
    let input = runtime.shell_session().unwrap().input.borrow();
    assert_eq!(
        input.last().map(Vec::as_slice),
        Some(b"tmux attach-session -t delta\r".as_slice())
    );
}

#[test]
fn runtime_start_session_action_reports_skipped_attach_without_shell() {
    let snapshot = TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState::default(),
        current: None,
    };
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 160,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    runtime.toggle_tmux_manager_panel(snapshot);
    let runner = FakeRunner::new(vec![ExpectedCall::success(&[
        "new-session",
        "-d",
        "-s",
        "delta",
    ])]);

    assert_eq!(
        runtime.handle_tmux_manager_key(&Key::Character("t".into()), ModifiersState::empty()),
        TmuxManagerKeyOutcome::Consumed
    );
    for character in "delta".chars() {
        assert_eq!(
            runtime.handle_tmux_manager_key(
                &Key::Character(character.to_string().into()),
                ModifiersState::empty()
            ),
            TmuxManagerKeyOutcome::Consumed
        );
    }
    let outcome =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty());
    let result = runtime
        .dispatch_tmux_manager_action(outcome, &runner)
        .unwrap();

    assert!(matches!(
        result,
        TmuxActionResult::Success {
            action_id: ActionId::StartSession,
            ..
        }
    ));
    runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Escape), ModifiersState::empty());
    let mut renderer = MockFrameRenderer::default();

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());

    let frame = renderer.frames.last().unwrap();
    assert!(
        frame.lines[7].contains("attach skipped: shell not started"),
        "{:?}",
        frame.lines[7]
    );
}
