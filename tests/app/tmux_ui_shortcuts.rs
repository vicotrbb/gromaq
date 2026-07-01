use gromaq::app::{TmuxManagerKeyOutcome, TmuxManagerPanelState};
use gromaq::tmux::{
    ActionId, TmuxManagerCurrent, TmuxManagerSnapshot, TmuxManagerStatus, TmuxPane, TmuxSession,
    TmuxState, TmuxWindow,
};
use winit::keyboard::{Key, ModifiersState};

#[test]
fn tmux_manager_panel_shortcuts_zoom_selected_pane() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);

    assert_eq!(
        panel.handle_key(
            &Key::Character("z".into()),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::ActionRequested(ActionId::ZoomPane)
    );
    assert_eq!(panel.pending_action(), Some("zoom-pane"));
}

#[test]
fn tmux_manager_panel_shortcuts_switch_windows() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);

    assert_eq!(
        panel.handle_key(
            &Key::Character("n".into()),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::ActionRequested(ActionId::NextWindow)
    );
    assert_eq!(panel.pending_action(), Some("next-window"));

    assert_eq!(
        panel.handle_key(
            &Key::Character("p".into()),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::ActionRequested(ActionId::PreviousWindow)
    );
    assert_eq!(panel.pending_action(), Some("previous-window"));
}

#[test]
fn tmux_manager_panel_shortcuts_attach_and_detach_sessions() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);

    assert_eq!(
        panel.handle_key(
            &Key::Character("a".into()),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::ActionRequested(ActionId::AttachSession)
    );
    assert_eq!(panel.pending_action(), Some("attach-session"));

    assert_eq!(
        panel.handle_key(
            &Key::Character("d".into()),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::ActionRequested(ActionId::DetachSession)
    );
    assert_eq!(panel.pending_action(), Some("detach-session"));
}

#[test]
fn tmux_manager_panel_shortcuts_start_session_name_input() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);

    assert_eq!(
        panel.handle_key(
            &Key::Character("t".into()),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::Consumed
    );
    assert_eq!(
        panel.action_input_prompt(),
        Some("start-session name: ".to_owned())
    );
}

#[test]
fn tmux_manager_panel_shortcuts_rename_with_name_input() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);

    assert_eq!(
        panel.handle_key(
            &Key::Character("e".into()),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::Consumed
    );
    assert_eq!(
        panel.action_input_prompt(),
        Some("rename-window name: ".to_owned())
    );

    panel.handle_key(
        &Key::Character("n".into()),
        ModifiersState::empty(),
        &snapshot,
    );
    assert_eq!(
        panel.action_input_prompt(),
        Some("rename-window name: n".to_owned())
    );
}

#[test]
fn tmux_manager_panel_shortcuts_rename_session_with_name_input() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);

    assert_eq!(
        panel.handle_key(
            &Key::Character("u".into()),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::Consumed
    );
    assert_eq!(
        panel.action_input_prompt(),
        Some("rename-session name: ".to_owned())
    );
}

#[test]
fn tmux_manager_panel_shortcuts_kill_session_requires_confirmation() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);

    assert_eq!(
        panel.handle_key(
            &Key::Character("q".into()),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::ConfirmationRequired(ActionId::KillSession)
    );
    assert_eq!(
        panel.confirmation_message(),
        Some("confirm kill-session with y, n/Esc cancels")
    );
}

#[test]
fn tmux_manager_panel_shortcuts_open_help() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);

    assert_eq!(
        panel.handle_key(
            &Key::Character("?".into()),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::ActionRequested(ActionId::ShowHelp)
    );
    assert_eq!(panel.pending_action(), Some("show-help"));
}

#[test]
fn tmux_manager_panel_shortcuts_request_refresh() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);

    assert_eq!(
        panel.handle_key(
            &Key::Character("r".into()),
            ModifiersState::empty(),
            &snapshot,
        ),
        TmuxManagerKeyOutcome::RefreshRequested
    );
}

fn manager_snapshot() -> TmuxManagerSnapshot {
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
            panes: vec![TmuxPane {
                session_name: "alpha".to_owned(),
                window_index: 1,
                index: 0,
                id: "%2".to_owned(),
                title: "editor".to_owned(),
                current_command: "nvim".to_owned(),
                active: true,
                width: Some(100),
                height: Some(30),
            }],
        },
        current: Some(TmuxManagerCurrent {
            session_name: "alpha".to_owned(),
            window_index: 1,
            pane_id: "%2".to_owned(),
        }),
    }
}
