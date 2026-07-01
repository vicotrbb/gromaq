//! Snapshot fixtures for native tmux UI render smoke checks.

use crate::tmux::{
    TmuxManagerCurrent, TmuxManagerSnapshot, TmuxManagerStatus, TmuxPane, TmuxSession, TmuxState,
    TmuxWindow,
};

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

pub(super) fn current_target_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState {
            sessions: vec![TmuxSession {
                name: "alpha".to_owned(),
                attached: true,
            }],
            windows: vec![TmuxWindow {
                session_name: "alpha".to_owned(),
                index: 1,
                name: "code".to_owned(),
                active: true,
            }],
            panes: vec![
                TmuxPane {
                    session_name: "alpha".to_owned(),
                    window_index: 1,
                    index: 0,
                    id: "%1".to_owned(),
                    title: "shell".to_owned(),
                    current_command: "zsh".to_owned(),
                    active: false,
                    width: Some(100),
                    height: Some(30),
                },
                TmuxPane {
                    session_name: "alpha".to_owned(),
                    window_index: 1,
                    index: 1,
                    id: "%2".to_owned(),
                    title: "editor".to_owned(),
                    current_command: "nvim".to_owned(),
                    active: true,
                    width: Some(100),
                    height: Some(30),
                },
            ],
        },
        current: Some(TmuxManagerCurrent {
            session_name: "alpha".to_owned(),
            window_index: 1,
            pane_id: "%2".to_owned(),
        }),
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
