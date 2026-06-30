use std::cell::RefCell;

use gromaq::app::{
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerFocus, TmuxManagerKeyOutcome,
    TmuxManagerPanelState, TmuxWorkspaceUiPreset,
};
use gromaq::config::{TmuxWorkspaceSettings, TmuxWorkspaceWindowSettings};
use gromaq::tmux::{
    TmuxCommandFailure, TmuxCommandOutput, TmuxCommandRunner, TmuxError, TmuxManagerSnapshot,
    TmuxState, TmuxWorkspaceResult,
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

    fn command_failure(args: &[&'static str]) -> Self {
        Self {
            args: args.to_vec(),
            result: Err(TmuxError::Command(TmuxCommandFailure::new(
                args.iter().map(|arg| (*arg).to_owned()).collect(),
                1,
                "missing session".to_owned(),
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
fn tmux_manager_panel_renders_workspace_preset_summary() {
    let snapshot = TmuxManagerSnapshot {
        state: TmuxState::default(),
        current: None,
    };
    let panel = TmuxManagerPanelState::open_for_snapshot_with_workspaces(
        &snapshot,
        vec![workspace_preset()],
    );
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 88,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_tmux_manager_panel(&mut renderer, &snapshot, &panel)
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[6].contains("Workspaces gromaq*"));
    assert!(frame.lines[6].contains("session gromaq"));
    assert!(frame.lines[6].contains("root /repo"));
    assert!(frame.lines[6].contains("windows code(2) test(1)"));
}

#[test]
fn tmux_manager_panel_keyboard_launches_selected_workspace_preset() {
    let snapshot = TmuxManagerSnapshot {
        state: TmuxState::default(),
        current: None,
    };
    let mut panel = TmuxManagerPanelState::open_for_snapshot_with_workspaces(
        &snapshot,
        vec![workspace_preset()],
    );

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
    panel.handle_key(
        &Key::Named(NamedKey::ArrowRight),
        ModifiersState::empty(),
        &snapshot,
    );
    assert_eq!(panel.focus(), TmuxManagerFocus::Workspaces);

    assert_eq!(
        panel.handle_key(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::WorkspaceLaunchRequested
    );
}

#[test]
fn runtime_dispatches_workspace_launch_outcome_through_launcher() {
    let snapshot = TmuxManagerSnapshot {
        state: TmuxState::default(),
        current: None,
    };
    let mut runtime =
        NativeTerminalRuntime::<crate::support::MockPtySession>::new(NativeTerminalRuntimeConfig {
            terminal_cols: 88,
            terminal_rows: 8,
            ..NativeTerminalRuntimeConfig::default()
        })
        .unwrap();
    runtime.toggle_tmux_manager_panel_with_workspaces(snapshot, vec![workspace_preset()]);
    let runner = FakeRunner::new(vec![ExpectedCall::success(&[
        "has-session",
        "-t",
        "gromaq",
    ])]);

    let result = runtime
        .dispatch_tmux_manager_workspace(TmuxManagerKeyOutcome::WorkspaceLaunchRequested, &runner);

    assert_eq!(
        result,
        Some(Ok(TmuxWorkspaceResult::Existing {
            session: "gromaq".to_owned()
        }))
    );
    assert_eq!(runner.remaining_calls(), 0);
}

#[test]
fn runtime_workspace_launch_attaches_started_workspace_through_pty() {
    let snapshot = TmuxManagerSnapshot {
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
    runtime.toggle_tmux_manager_panel_with_workspaces(snapshot, vec![workspace_preset()]);
    let runner = FakeRunner::new(vec![
        ExpectedCall::command_failure(&["has-session", "-t", "gromaq"]),
        ExpectedCall::success(&[
            "new-session",
            "-d",
            "-s",
            "gromaq",
            "-n",
            "code",
            "-c",
            "/repo",
            "nvim",
        ]),
        ExpectedCall::success(&[
            "split-window",
            "-t",
            "gromaq:0",
            "-c",
            "/repo",
            "cargo test",
        ]),
        ExpectedCall::success(&[
            "new-window",
            "-t",
            "gromaq",
            "-n",
            "test",
            "-c",
            "/repo",
            "cargo watch",
        ]),
    ]);

    let result = runtime
        .dispatch_tmux_manager_workspace(TmuxManagerKeyOutcome::WorkspaceLaunchRequested, &runner);

    assert!(matches!(
        result,
        Some(Ok(TmuxWorkspaceResult::Started { .. }))
    ));
    assert_eq!(runner.remaining_calls(), 0);
    let input = runtime.shell_session().unwrap().input.borrow();
    assert_eq!(
        input.last().map(Vec::as_slice),
        Some(b"tmux attach-session -t gromaq\r".as_slice())
    );
}

#[test]
fn runtime_workspace_launch_attaches_existing_workspace_through_pty_without_runner_attach() {
    let snapshot = TmuxManagerSnapshot {
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
    runtime.toggle_tmux_manager_panel_with_workspaces(snapshot, vec![workspace_preset()]);
    let runner = FakeRunner::new(vec![ExpectedCall::success(&[
        "has-session",
        "-t",
        "gromaq",
    ])]);

    let result = runtime
        .dispatch_tmux_manager_workspace(TmuxManagerKeyOutcome::WorkspaceLaunchRequested, &runner);

    assert_eq!(
        result,
        Some(Ok(TmuxWorkspaceResult::Existing {
            session: "gromaq".to_owned()
        }))
    );
    assert_eq!(runner.remaining_calls(), 0);
    let input = runtime.shell_session().unwrap().input.borrow();
    assert_eq!(
        input.last().map(Vec::as_slice),
        Some(b"tmux attach-session -t gromaq\r".as_slice())
    );
}

#[test]
fn tmux_manager_panel_workspace_launch_attaches_existing_session_without_duplication() {
    let snapshot = TmuxManagerSnapshot {
        state: TmuxState::default(),
        current: None,
    };
    let mut panel = TmuxManagerPanelState::open_for_snapshot_with_workspaces(
        &snapshot,
        vec![workspace_preset()],
    );
    let runner = FakeRunner::new(vec![
        ExpectedCall::success(&["has-session", "-t", "gromaq"]),
        ExpectedCall::success(&["attach-session", "-t", "gromaq"]),
    ]);

    let result = panel.launch_selected_workspace(&runner).unwrap().unwrap();

    assert_eq!(
        result,
        TmuxWorkspaceResult::Attached {
            session: "gromaq".to_owned()
        }
    );
    assert_eq!(runner.remaining_calls(), 0);
    assert_eq!(
        panel.workspace_feedback(),
        Some("workspace gromaq attached session gromaq")
    );
}

#[test]
fn tmux_manager_panel_invalid_workspace_does_not_launch_and_reports_feedback() {
    let snapshot = TmuxManagerSnapshot {
        state: TmuxState::default(),
        current: None,
    };
    let mut panel = TmuxManagerPanelState::open_for_snapshot_with_workspaces(
        &snapshot,
        vec![TmuxWorkspaceUiPreset::new(
            "bad",
            TmuxWorkspaceSettings::default(),
        )],
    );
    let runner = FakeRunner::new(Vec::new());

    let result = panel.launch_selected_workspace(&runner).unwrap();

    assert!(matches!(
        result,
        Err(TmuxError::InvalidWorkspace {
            workspace,
            reason: "session is empty",
        }) if workspace == "bad"
    ));
    assert_eq!(runner.remaining_calls(), 0);
    assert_eq!(
        panel.workspace_feedback(),
        Some("workspace bad invalid: session is empty")
    );
}

#[test]
fn tmux_manager_panel_workspace_launch_starts_absent_session() {
    let snapshot = TmuxManagerSnapshot {
        state: TmuxState::default(),
        current: None,
    };
    let mut panel = TmuxManagerPanelState::open_for_snapshot_with_workspaces(
        &snapshot,
        vec![workspace_preset()],
    );
    let runner = FakeRunner::new(vec![
        ExpectedCall::command_failure(&["has-session", "-t", "gromaq"]),
        ExpectedCall::success(&[
            "new-session",
            "-d",
            "-s",
            "gromaq",
            "-n",
            "code",
            "-c",
            "/repo",
            "nvim",
        ]),
        ExpectedCall::success(&[
            "split-window",
            "-t",
            "gromaq:0",
            "-c",
            "/repo",
            "cargo test",
        ]),
        ExpectedCall::success(&[
            "new-window",
            "-t",
            "gromaq",
            "-n",
            "test",
            "-c",
            "/repo",
            "cargo watch",
        ]),
    ]);

    let result = panel.launch_selected_workspace(&runner).unwrap().unwrap();

    assert_eq!(
        result,
        TmuxWorkspaceResult::Started {
            session: "gromaq".to_owned(),
            windows: 2,
            panes: 3,
        }
    );
    assert_eq!(runner.remaining_calls(), 0);
    assert_eq!(
        panel.workspace_feedback(),
        Some("workspace gromaq started session gromaq windows 2 panes 3")
    );
}

fn workspace_preset() -> TmuxWorkspaceUiPreset {
    TmuxWorkspaceUiPreset::new(
        "gromaq",
        TmuxWorkspaceSettings {
            session: "gromaq".to_owned(),
            root: Some("/repo".to_owned()),
            windows: vec![
                TmuxWorkspaceWindowSettings {
                    name: "code".to_owned(),
                    panes: vec!["nvim".to_owned(), "cargo test".to_owned()],
                },
                TmuxWorkspaceWindowSettings {
                    name: "test".to_owned(),
                    panes: vec!["cargo watch".to_owned()],
                },
            ],
        },
    )
}
