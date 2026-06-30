//! Read-only tmux manager snapshot primitives.

use super::{
    SystemTmuxCommandRunner, TmuxCommandRunner, TmuxError, TmuxPane, TmuxSession, TmuxState,
    TmuxStateReader, TmuxWindow,
};

const CURRENT_FORMAT: &str = "#{session_name}\t#{window_index}\t#{pane_id}";

/// Current tmux target reported by the active client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxManagerCurrent {
    /// Active session name.
    pub session_name: String,
    /// Active window index.
    pub window_index: u16,
    /// Active pane id.
    pub pane_id: String,
}

/// Full manager snapshot for native manager views and CLI diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxManagerSnapshot {
    /// Read-only tmux server state.
    pub state: TmuxState,
    /// Current target when a client context is available.
    pub current: Option<TmuxManagerCurrent>,
}

impl TmuxManagerSnapshot {
    /// Return the current session when tmux client metadata matches state.
    pub fn current_session(&self) -> Option<&TmuxSession> {
        let current = self.current.as_ref()?;
        self.state
            .sessions
            .iter()
            .find(|session| session.name == current.session_name)
    }

    /// Return windows belonging to the current session.
    pub fn current_windows(&self) -> Vec<&TmuxWindow> {
        let Some(current) = self.current.as_ref() else {
            return Vec::new();
        };
        self.state
            .windows
            .iter()
            .filter(|window| window.session_name == current.session_name)
            .collect()
    }

    /// Return panes belonging to the current window.
    pub fn current_window_panes(&self) -> Vec<&TmuxPane> {
        let Some(current) = self.current.as_ref() else {
            return Vec::new();
        };
        self.state
            .panes
            .iter()
            .filter(|pane| {
                pane.session_name == current.session_name
                    && pane.window_index == current.window_index
            })
            .collect()
    }
}

/// Read-only tmux manager backed by the tmux CLI.
#[derive(Debug, Clone)]
pub struct TmuxManager<R = SystemTmuxCommandRunner> {
    runner: R,
}

impl<R> TmuxManager<R>
where
    R: TmuxCommandRunner,
{
    /// Create a manager backed by a tmux command runner.
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    /// Read tmux state and best-effort current target metadata.
    pub fn snapshot(&self) -> Result<TmuxManagerSnapshot, TmuxError> {
        let state = TmuxStateReader::new(&self.runner).read_state()?;
        Ok(TmuxManagerSnapshot {
            state,
            current: self.current().ok(),
        })
    }

    /// Read the current target from tmux client context.
    pub fn current(&self) -> Result<TmuxManagerCurrent, TmuxError> {
        let output = self
            .runner
            .run_tmux(&["display-message", "-p", CURRENT_FORMAT])?;
        parse_current(&output.stdout)
    }
}

fn parse_current(output: &str) -> Result<TmuxManagerCurrent, TmuxError> {
    let row = output.trim_end_matches(['\r', '\n']);
    let fields = row.split('\t').collect::<Vec<_>>();
    if fields.len() != 3 || fields.iter().any(|field| field.is_empty()) {
        return Err(TmuxError::Parse {
            context: "current target row",
            row: row.to_owned(),
        });
    }
    Ok(TmuxManagerCurrent {
        session_name: fields[0].to_owned(),
        window_index: fields[1].parse().map_err(|_| TmuxError::Parse {
            context: "current window index",
            row: fields[1].to_owned(),
        })?,
        pane_id: fields[2].to_owned(),
    })
}
