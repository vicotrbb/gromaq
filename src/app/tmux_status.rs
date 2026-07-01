use crate::app::{TmuxStatusKind, TmuxUiSnapshot};
use crate::tmux::{
    SystemTmuxCommandRunner, TmuxError, TmuxManager, TmuxManagerSnapshot, TmuxProbe,
};

pub(super) fn read_tmux_status_snapshot() -> TmuxUiSnapshot {
    let runner = SystemTmuxCommandRunner;
    match TmuxProbe::new(runner).probe() {
        Ok(status) if !status.installed => empty_status(TmuxStatusKind::Missing),
        Err(TmuxError::Missing) => empty_status(TmuxStatusKind::Missing),
        Err(_) => empty_status(TmuxStatusKind::NoServer),
        Ok(_) => match TmuxManager::new(runner).snapshot() {
            Ok(snapshot) => TmuxUiSnapshot::from_manager_snapshot(&snapshot),
            Err(TmuxError::Missing) => empty_status(TmuxStatusKind::Missing),
            Err(_) => empty_status(TmuxStatusKind::NoServer),
        },
    }
}

pub(super) fn read_tmux_manager_snapshot() -> TmuxManagerSnapshot {
    TmuxManager::new(SystemTmuxCommandRunner)
        .snapshot()
        .unwrap_or_else(|error| match error {
            TmuxError::Missing => TmuxManagerSnapshot::missing(),
            _ => TmuxManagerSnapshot::no_server(),
        })
}

fn empty_status(status: TmuxStatusKind) -> TmuxUiSnapshot {
    TmuxUiSnapshot {
        status,
        current_session: None,
        current_window: None,
        visible_windows: Vec::new(),
        pane_count: None,
        active_pane_id: None,
        active_pane_command: None,
        pending_feedback: None,
        confirmation_feedback: None,
    }
}
