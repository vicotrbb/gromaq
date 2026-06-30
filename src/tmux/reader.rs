//! Read-only tmux state discovery.

use super::{SystemTmuxCommandRunner, TmuxCommandRunner, TmuxError, TmuxState};

const SESSION_FORMAT: &str = "#{session_name}\t#{session_attached}";
const WINDOW_FORMAT: &str = "#{session_name}\t#{window_index}\t#{window_name}\t#{window_active}";
const PANE_FORMAT: &str = "#{session_name}\t#{window_index}\t#{pane_index}\t#{pane_id}\t#{pane_title}\t#{pane_current_command}\t#{pane_active}\t#{pane_width}\t#{pane_height}";

/// Read-only tmux state reader backed by the tmux CLI.
#[derive(Debug, Clone)]
pub struct TmuxStateReader<R = SystemTmuxCommandRunner> {
    runner: R,
}

impl<R> TmuxStateReader<R>
where
    R: TmuxCommandRunner,
{
    /// Create a state reader backed by a tmux command runner.
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    /// Read sessions, windows, and panes using stable tmux format strings.
    pub fn read_state(&self) -> Result<TmuxState, TmuxError> {
        let sessions = self
            .runner
            .run_tmux(&["list-sessions", "-F", SESSION_FORMAT])?;
        let windows = self
            .runner
            .run_tmux(&["list-windows", "-a", "-F", WINDOW_FORMAT])?;
        let panes = self
            .runner
            .run_tmux(&["list-panes", "-a", "-F", PANE_FORMAT])?;
        TmuxState::parse(&sessions.stdout, &windows.stdout, &panes.stdout)
    }
}
