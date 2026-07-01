//! Isolated tmux action and state-reader runtime smoke.

mod availability;
mod ui;

use crate::cli::CliExit;
use crate::tmux::{
    ActionId, SocketTmuxCommandRunner, SystemTmuxCommandRunner, TmuxActionRequest,
    TmuxActionResult, TmuxActionRunner, TmuxCommandRunner, TmuxProbe, TmuxStateReader,
};
use availability::tmux_missing_skip_exit;
pub(super) use ui::runtime_tmux_ui_smoke_exit;

const SESSION: &str = "gromaq-runtime-tmux";

pub(super) fn runtime_tmux_smoke_exit() -> CliExit {
    let probe = match TmuxProbe::new(SystemTmuxCommandRunner).probe() {
        Ok(probe) => probe,
        Err(error) => return failure(format!("tmux probe failed: {error:?}")),
    };
    if !probe.installed {
        return tmux_missing_skip_exit("runtime tmux smoke");
    }

    let socket = format!("gromaq-runtime-tmux-{}", std::process::id());
    let runner = SocketTmuxCommandRunner::new(socket.clone());
    let mut cleanup = TmuxSmokeCleanup::new(runner.clone());
    if let Err(error) = runner.run_tmux(&["new-session", "-d", "-s", SESSION]) {
        return failure(format!("create isolated tmux session failed: {error:?}"));
    }

    let action_runner = TmuxActionRunner::new(runner.clone());
    let split = action_runner
        .run(TmuxActionRequest::new(ActionId::SplitPaneRight).target(format!("{SESSION}:0")));
    if !matches!(split, TmuxActionResult::Success { .. }) {
        return failure(format!("split pane action failed: {split:?}"));
    }
    let new_window = action_runner.run(
        TmuxActionRequest::new(ActionId::NewWindow)
            .target(SESSION)
            .new_name("logs"),
    );
    if !matches!(new_window, TmuxActionResult::Success { .. }) {
        return failure(format!("new window action failed: {new_window:?}"));
    }

    let state = match TmuxStateReader::new(runner.clone()).read_state() {
        Ok(state) => state,
        Err(error) => return failure(format!("tmux state reader failed: {error:?}")),
    };
    let observed_session = state.sessions.iter().any(|session| session.name == SESSION);
    let cleanup_ok = cleanup.kill_server();

    CliExit {
        code: 0,
        stdout: format!(
            "runtime tmux smoke: ok\ntmux available: true\nsocket: {socket}\nsession: {SESSION}\ncreated session: true\nsplit pane action: success\nnew window action: success\nstate sessions: {}\nstate windows: {}\nstate panes: {}\nstate reader observed session: {observed_session}\ncleanup killed session: {cleanup_ok}\n",
            state.sessions.len(),
            state.windows.len(),
            state.panes.len()
        ),
        stderr: String::new(),
    }
}

fn failure(message: String) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime tmux smoke failed: {message}\n"),
    }
}

struct TmuxSmokeCleanup {
    runner: SocketTmuxCommandRunner,
    cleaned: bool,
}

impl TmuxSmokeCleanup {
    fn new(runner: SocketTmuxCommandRunner) -> Self {
        Self {
            runner,
            cleaned: false,
        }
    }

    fn kill_server(&mut self) -> bool {
        self.cleaned = true;
        self.runner.run_tmux(&["kill-server"]).is_ok()
    }
}

impl Drop for TmuxSmokeCleanup {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.runner.run_tmux(&["kill-server"]);
        }
    }
}
