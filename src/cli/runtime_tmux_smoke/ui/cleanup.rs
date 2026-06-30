//! Cleanup guard for isolated tmux UI smoke sockets.

use crate::tmux::{SocketTmuxCommandRunner, TmuxCommandRunner};

pub(super) struct TmuxUiSmokeCleanup {
    runner: SocketTmuxCommandRunner,
    cleaned: bool,
}

impl TmuxUiSmokeCleanup {
    pub(super) fn new(runner: SocketTmuxCommandRunner) -> Self {
        Self {
            runner,
            cleaned: false,
        }
    }

    pub(super) fn kill_server(&mut self) -> bool {
        self.cleaned = true;
        self.runner.run_tmux(&["kill-server"]).is_ok()
    }
}

impl Drop for TmuxUiSmokeCleanup {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.runner.run_tmux(&["kill-server"]);
        }
    }
}
