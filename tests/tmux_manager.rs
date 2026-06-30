use std::cell::RefCell;

use gromaq::tmux::{
    TmuxCommandOutput, TmuxCommandRunner, TmuxError, TmuxManager, TmuxManagerCurrent,
};

#[derive(Debug, Clone)]
struct FakeRunner {
    calls: RefCell<Vec<ExpectedCall>>,
}

#[derive(Debug, Clone)]
struct ExpectedCall {
    args: Vec<&'static str>,
    stdout: &'static str,
}

impl FakeRunner {
    fn new(calls: Vec<ExpectedCall>) -> Self {
        Self {
            calls: RefCell::new(calls),
        }
    }
}

impl ExpectedCall {
    fn output(args: &[&'static str], stdout: &'static str) -> Self {
        Self {
            args: args.to_vec(),
            stdout,
        }
    }
}

impl TmuxCommandRunner for FakeRunner {
    fn run_tmux(&self, args: &[&str]) -> Result<TmuxCommandOutput, TmuxError> {
        let expected = self.calls.borrow_mut().remove(0);
        assert_eq!(args, expected.args.as_slice());
        Ok(TmuxCommandOutput::new(
            expected.stdout.to_owned(),
            String::new(),
        ))
    }
}

#[test]
fn tmux_manager_snapshot_reads_state_and_current_target() {
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
            "alpha\t0\tcode\t1\nalpha\t1\tlogs\t0\n",
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
        ExpectedCall::output(
            &[
                "display-message",
                "-p",
                "#{session_name}\t#{window_index}\t#{pane_id}",
            ],
            "alpha\t0\t%1\n",
        ),
    ]);

    let snapshot = TmuxManager::new(runner).snapshot().unwrap();

    assert_eq!(snapshot.state.sessions[0].name, "alpha");
    assert_eq!(snapshot.state.windows.len(), 2);
    assert_eq!(
        snapshot.current,
        Some(TmuxManagerCurrent {
            session_name: "alpha".to_owned(),
            window_index: 0,
            pane_id: "%1".to_owned(),
        })
    );
}

#[test]
fn tmux_manager_snapshot_filters_current_scope() {
    let state = gromaq::tmux::TmuxState::parse(
        "alpha\t1\nbeta\t0\n",
        "alpha\t0\tcode\t1\nalpha\t1\tlogs\t0\nbeta\t0\tother\t1\n",
        "alpha\t0\t0\t%1\tshell\tzsh\t1\t120\t36\nalpha\t1\t0\t%2\tlogs\ttail\t1\t120\t36\n",
    )
    .unwrap();
    let snapshot = gromaq::tmux::TmuxManagerSnapshot {
        state,
        current: Some(TmuxManagerCurrent {
            session_name: "alpha".to_owned(),
            window_index: 0,
            pane_id: "%1".to_owned(),
        }),
    };

    assert_eq!(snapshot.current_session().unwrap().name, "alpha");
    assert_eq!(snapshot.current_windows().len(), 2);
    assert_eq!(snapshot.current_window_panes().len(), 1);
    assert_eq!(snapshot.current_window_panes()[0].id, "%1");
}
