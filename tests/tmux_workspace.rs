use std::cell::RefCell;

use gromaq::tmux::{
    TmuxCommandFailure, TmuxCommandOutput, TmuxCommandRunner, TmuxError, TmuxWorkspaceLauncher,
    TmuxWorkspaceResult,
};
use gromaq::{TmuxWorkspaceSettings, TmuxWorkspaceWindowSettings};

#[derive(Debug, Clone)]
struct FakeRunner {
    calls: RefCell<Vec<ExpectedCall>>,
}

#[derive(Debug, Clone)]
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
    fn output(args: &[&'static str], stdout: &str) -> Self {
        Self {
            args: args.to_vec(),
            result: Ok(TmuxCommandOutput::new(stdout.to_owned(), String::new())),
        }
    }

    fn command_failure(args: &[&'static str]) -> Self {
        Self {
            args: args.to_vec(),
            result: Err(TmuxError::Command(TmuxCommandFailure {
                args: args.iter().map(|arg| (*arg).to_owned()).collect(),
                exit_code: Some(1),
                stderr: "missing session".to_owned(),
            })),
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
fn tmux_workspace_launcher_starts_absent_workspace_with_structured_args() {
    let workspace = TmuxWorkspaceSettings {
        session: "gromaq".to_owned(),
        root: Some("/repo/gromaq".to_owned()),
        windows: vec![
            TmuxWorkspaceWindowSettings {
                name: "code".to_owned(),
                panes: vec!["$SHELL".to_owned(), "cargo test --all".to_owned()],
            },
            TmuxWorkspaceWindowSettings {
                name: "logs".to_owned(),
                panes: vec!["cargo run -- --runtime-tmux-smoke".to_owned()],
            },
        ],
    };
    let runner = FakeRunner::new(vec![
        ExpectedCall::command_failure(&["has-session", "-t", "gromaq"]),
        ExpectedCall::output(
            &[
                "new-session",
                "-d",
                "-s",
                "gromaq",
                "-n",
                "code",
                "-c",
                "/repo/gromaq",
                "$SHELL",
            ],
            "",
        ),
        ExpectedCall::output(
            &[
                "split-window",
                "-t",
                "gromaq:0",
                "-c",
                "/repo/gromaq",
                "cargo test --all",
            ],
            "",
        ),
        ExpectedCall::output(
            &[
                "new-window",
                "-t",
                "gromaq",
                "-n",
                "logs",
                "-c",
                "/repo/gromaq",
                "cargo run -- --runtime-tmux-smoke",
            ],
            "",
        ),
    ]);
    let result = TmuxWorkspaceLauncher::new(runner)
        .start_or_attach("gromaq", &workspace)
        .unwrap();
    assert_eq!(
        result,
        TmuxWorkspaceResult::Started {
            session: "gromaq".to_owned(),
            windows: 2,
            panes: 3,
        }
    );
}

#[test]
fn tmux_workspace_launcher_attaches_existing_workspace_without_duplication() {
    let workspace = TmuxWorkspaceSettings {
        session: "gromaq".to_owned(),
        root: None,
        windows: vec![TmuxWorkspaceWindowSettings {
            name: "code".to_owned(),
            panes: vec!["$SHELL".to_owned()],
        }],
    };
    let runner = FakeRunner::new(vec![
        ExpectedCall::output(&["has-session", "-t", "gromaq"], ""),
        ExpectedCall::output(&["attach-session", "-t", "gromaq"], ""),
    ]);
    let result = TmuxWorkspaceLauncher::new(runner)
        .start_or_attach("gromaq", &workspace)
        .unwrap();
    assert_eq!(
        result,
        TmuxWorkspaceResult::Attached {
            session: "gromaq".to_owned(),
        }
    );
}

#[test]
fn tmux_workspace_launcher_can_ensure_existing_workspace_without_attaching() {
    let workspace = TmuxWorkspaceSettings {
        session: "gromaq".to_owned(),
        root: None,
        windows: vec![TmuxWorkspaceWindowSettings {
            name: "code".to_owned(),
            panes: vec!["$SHELL".to_owned()],
        }],
    };
    let runner = FakeRunner::new(vec![ExpectedCall::output(
        &["has-session", "-t", "gromaq"],
        "",
    )]);
    let result = TmuxWorkspaceLauncher::new(&runner)
        .start_if_absent("gromaq", &workspace)
        .unwrap();

    assert_eq!(
        result,
        TmuxWorkspaceResult::Existing {
            session: "gromaq".to_owned(),
        }
    );
    assert_eq!(runner.remaining_calls(), 0);
}
