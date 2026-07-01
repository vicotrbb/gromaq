use gromaq::app::{TmuxManagerFocus, TmuxManagerKeyOutcome, TmuxManagerPanelState};
use gromaq::tmux::{
    ActionId, TmuxManagerCurrent, TmuxManagerSnapshot, TmuxManagerStatus, TmuxPane, TmuxSession,
    TmuxState, TmuxWindow,
};
use winit::keyboard::{Key, ModifiersState, NamedKey};

#[test]
fn tmux_manager_panel_wraps_selection_navigation() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);

    assert_eq!(panel.selected_session_name(&snapshot), Some("alpha"));
    assert_eq!(
        panel.handle_key(
            &Key::Named(NamedKey::ArrowUp),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::Consumed
    );
    assert_eq!(panel.selected_session_name(&snapshot), Some("beta"));
    assert_eq!(
        panel.handle_key(
            &Key::Named(NamedKey::ArrowDown),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::Consumed
    );
    assert_eq!(panel.selected_session_name(&snapshot), Some("alpha"));

    panel.focus_next();
    panel.focus_next();
    panel.focus_next();
    assert_eq!(panel.focus(), TmuxManagerFocus::Actions);
    assert_eq!(
        panel.handle_key(
            &Key::Named(NamedKey::ArrowUp),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::Consumed
    );
    assert_eq!(
        panel.handle_key(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::ActionRequested(ActionId::ShowHelp)
    );
}

#[test]
fn tmux_manager_panel_enter_selects_focused_pane() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    panel.focus_next();
    panel.focus_next();

    assert_eq!(panel.focus(), TmuxManagerFocus::Panes);
    assert_eq!(
        panel.handle_key(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty(),
            &snapshot
        ),
        TmuxManagerKeyOutcome::ActionRequested(ActionId::SelectPane)
    );
    assert_eq!(panel.pending_action(), Some("select-pane"));
}

fn manager_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState {
            sessions: vec![
                TmuxSession {
                    name: "alpha".to_owned(),
                    attached: true,
                },
                TmuxSession {
                    name: "beta".to_owned(),
                    attached: false,
                },
            ],
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
