use gromaq::tmux::{
    ActionId, TmuxAction, TmuxActionRequest, TmuxActionResult, TmuxActionRunner,
    TmuxCommandFailure, TmuxCommandOutput, TmuxCommandRunner, TmuxError, TmuxProbe, TmuxState,
    TmuxStateReader, TmuxTerminalCommand, TmuxWorkspaceLauncher, TmuxWorkspaceResult,
};
use gromaq::{TmuxWorkspaceSettings, TmuxWorkspaceWindowSettings};
use std::cell::RefCell;

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
    fn missing_tmux() -> Self {
        Self::new(vec![ExpectedCall::error(&["-V"], TmuxError::Missing)])
    }

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

    fn error(args: &[&'static str], error: TmuxError) -> Self {
        Self {
            args: args.to_vec(),
            result: Err(error),
        }
    }

    fn command_failure(args: &[&'static str]) -> Self {
        Self::error(
            args,
            TmuxError::Command(TmuxCommandFailure {
                args: args.iter().map(|arg| (*arg).to_owned()).collect(),
                exit_code: Some(1),
                stderr: "missing session".to_owned(),
            }),
        )
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
fn tmux_version_parser_accepts_patch_suffix() {
    let version = TmuxProbe::parse_version("tmux 3.5a\n").unwrap();
    assert_eq!(version.major, 3);
    assert_eq!(version.minor, 5);
    assert_eq!(version.patch, None);
    assert_eq!(version.suffix, "a");
    assert_eq!(version.raw, "tmux 3.5a");
}

#[test]
fn tmux_probe_reports_missing_binary_without_failing() {
    let status = TmuxProbe::new(FakeRunner::missing_tmux()).probe().unwrap();
    assert!(!status.installed);
    assert_eq!(status.version, None);
    assert!(!status.inside_tmux);
    assert!(!status.attachable_sessions);
}

#[test]
fn tmux_probe_detects_attachable_sessions() {
    let runner = FakeRunner::new(vec![
        ExpectedCall::output(&["-V"], "tmux 3.5a\n"),
        ExpectedCall::output(&["list-sessions", "-F", "#{session_name}"], "alpha\n"),
    ]);
    let status = TmuxProbe::new(runner).probe().unwrap();
    assert!(status.installed);
    assert!(status.attachable_sessions);
}

#[test]
fn tmux_state_reader_uses_stable_list_formats() {
    let runner = FakeRunner::new(vec![
        ExpectedCall::output(
            &[
                "list-sessions",
                "-F",
                "#{session_name}\t#{session_attached}",
            ],
            "alpha\t1\n",
        ),
        ExpectedCall::output(
            &[
                "list-windows",
                "-a",
                "-F",
                "#{session_name}\t#{window_index}\t#{window_name}\t#{window_active}",
            ],
            "alpha\t0\tcode\t1\n",
        ),
        ExpectedCall::output(
            &[
                "list-panes",
                "-a",
                "-F",
                "#{session_name}\t#{window_index}\t#{pane_index}\t#{pane_id}\t#{pane_title}\t#{pane_current_command}\t#{pane_active}\t#{pane_width}\t#{pane_height}",
            ],
            "alpha\t0\t0\t%1\tshell\tzsh\t1\t120\t36\n",
        ),
    ]);
    let state = TmuxStateReader::new(runner).read_state().unwrap();
    assert_eq!(state.sessions[0].name, "alpha");
    assert_eq!(state.windows[0].name, "code");
    assert_eq!(state.panes[0].id, "%1");
}

#[test]
fn tmux_state_parser_reads_sessions_windows_and_panes() {
    let state = TmuxState::parse(
        "alpha\t1\nwork|space\t0\n",
        "alpha\t0\tcode\t1\nalpha\t1\ttest\t0\n",
        "alpha\t0\t0\t%1\tshell\tzsh\t1\t120\t36\n",
    )
    .unwrap();
    assert_eq!(state.sessions.len(), 2);
    assert_eq!(state.sessions[0].name, "alpha");
    assert!(state.sessions[0].attached);
    assert_eq!(state.sessions[1].name, "work|space");
    assert!(!state.sessions[1].attached);
    assert_eq!(state.windows[1].name, "test");
    assert!(!state.windows[1].active);
    assert_eq!(state.panes[0].id, "%1");
    assert_eq!(state.panes[0].current_command, "zsh");
    assert_eq!(state.panes[0].width, Some(120));
    assert_eq!(state.panes[0].height, Some(36));
}

#[test]
fn tmux_state_parser_rejects_malformed_rows() {
    let error = TmuxState::parse("alpha\t1\textra\n", "", "").unwrap_err();
    assert!(matches!(error, TmuxError::Parse { .. }));
}

#[test]
fn tmux_action_registry_describes_teaching_and_safety_metadata() {
    let split = TmuxAction::by_id(ActionId::SplitPaneRight).unwrap();
    let kill_session = TmuxAction::by_id(ActionId::KillSession).unwrap();
    assert_eq!(split.label, "Split pane right");
    assert_eq!(split.tmux_command, "tmux split-window -h");
    assert_eq!(split.key_binding, Some("Ctrl-b %"));
    assert!(!split.destructive);
    assert!(!split.confirmation_required);
    assert!(split.requires_active_tmux);
    assert_eq!(kill_session.tmux_command, "tmux kill-session -t <session>");
    assert!(kill_session.destructive);
    assert!(kill_session.confirmation_required);
    assert!(!kill_session.can_run_outside_tmux);
}

#[test]
fn tmux_action_registry_finds_actions_by_stable_id() {
    assert_eq!(
        TmuxAction::by_stable_id("split-pane-right").unwrap().id,
        ActionId::SplitPaneRight
    );
    assert!(TmuxAction::by_stable_id("missing-action").is_none());
}

#[test]
fn tmux_terminal_command_renders_attach_as_pty_input() {
    let input = TmuxTerminalCommand::attach_session("alpha").to_pty_input();

    assert_eq!(input, b"tmux attach-session -t alpha\r");
}

#[test]
fn tmux_terminal_command_quotes_shell_metacharacters_as_one_argument() {
    let input = TmuxTerminalCommand::attach_session("dev'; rm -rf / #").to_pty_input();

    assert_eq!(
        String::from_utf8(input).unwrap(),
        "tmux attach-session -t 'dev'\\''; rm -rf / #'\r"
    );
}

#[test]
fn tmux_command_failure_preserves_stderr() {
    let failure = TmuxCommandFailure::new(vec!["list-sessions".into()], 1, "no server".into());
    assert_eq!(failure.args, vec!["list-sessions"]);
    assert_eq!(failure.exit_code, Some(1));
    assert_eq!(failure.stderr, "no server");
}

#[test]
fn tmux_action_runner_executes_safe_action_with_teaching_hint() {
    let runner = FakeRunner::new(vec![ExpectedCall::output(&["split-window", "-h"], "")]);
    let result =
        TmuxActionRunner::new(runner).run(TmuxActionRequest::new(ActionId::SplitPaneRight));
    match result {
        TmuxActionResult::Success { teaching_hint, .. } => {
            assert!(teaching_hint.contains("tmux command: tmux split-window -h"));
            assert!(teaching_hint.contains("tmux key: Ctrl-b %"));
        }
        other => panic!("unexpected action result: {other:?}"),
    }
}

#[test]
fn tmux_action_runner_requires_confirmation_for_destructive_actions() {
    let runner = FakeRunner::new(Vec::new());
    let result = TmuxActionRunner::new(runner)
        .run(TmuxActionRequest::new(ActionId::KillSession).target("alpha"));

    assert!(matches!(
        result,
        TmuxActionResult::ConfirmationRequired {
            action_id: ActionId::KillSession,
            ..
        }
    ));
}

#[test]
fn tmux_action_runner_reports_no_active_session_for_current_session_actions() {
    let runner = FakeRunner::new(Vec::new());
    let result = TmuxActionRunner::new(runner)
        .run(TmuxActionRequest::new(ActionId::SplitPaneRight).active_tmux(false));
    assert!(matches!(
        result,
        TmuxActionResult::NoActiveSession {
            action_id: ActionId::SplitPaneRight,
            ..
        }
    ));
}

#[test]
fn tmux_action_runner_executes_confirmed_destructive_target() {
    let runner = FakeRunner::new(vec![ExpectedCall::output(
        &["kill-session", "-t", "alpha"],
        "",
    )]);
    let result = TmuxActionRunner::new(runner).run(
        TmuxActionRequest::new(ActionId::KillSession)
            .target("alpha")
            .confirmed(true),
    );
    assert!(matches!(result, TmuxActionResult::Success { .. }));
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
