//! Snapshot fixtures for native tmux UI render smoke checks.

use crate::tmux::{TmuxManagerSnapshot, TmuxManagerStatus, TmuxSession, TmuxState};

pub(super) fn no_server_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::NoServer,
        state: TmuxState::default(),
        current: None,
    }
}

pub(super) fn detached_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState {
            sessions: vec![TmuxSession {
                name: "gromaq-runtime-detached".to_owned(),
                attached: false,
            }],
            windows: Vec::new(),
            panes: Vec::new(),
        },
        current: None,
    }
}

pub(super) fn full_prompt_grid() -> String {
    (0..16)
        .map(|row| format!("gromaq startup line {row:02}\r\n"))
        .chain(std::iter::once(
            "Now using node v20.20.0 (npm v10.8.2)\r\n~ ................................ rb 2.7.5 22:17:47\r\n> ".to_owned(),
        ))
        .collect()
}
