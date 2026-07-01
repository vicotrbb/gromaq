use gromaq::app::{
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, NativeWindowMouseInput, TmuxManagerFocus,
    TmuxManagerMouseOutcome, TmuxManagerPanelState,
};
use gromaq::tmux::{
    TmuxManagerCurrent, TmuxManagerSnapshot, TmuxManagerStatus, TmuxPane, TmuxSession, TmuxState,
    TmuxWindow,
};
use gromaq::{MouseButton, MouseEvent, MouseEventKind};

use crate::support::{MockFrameRenderer, MockPtySession};

#[test]
fn runtime_mouse_press_focuses_rendered_tmux_manager_panel_row() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 72,
        terminal_rows: 8,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    runtime.toggle_tmux_manager_panel(manager_snapshot());
    let mut renderer = MockFrameRenderer::default();

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());
    let manager_region = renderer
        .frames
        .last()
        .unwrap()
        .dirty_regions
        .iter()
        .find(|region| region.rows == 6 && region.cols == 72)
        .copied()
        .expect("manager panel region should render");

    let result = runtime
        .send_window_mouse_input_event_result(NativeWindowMouseInput {
            x: 1.0,
            y: f64::from(manager_region.row + 2) * 10.0 + 1.0,
            window_width_px: 720,
            window_height_px: 80,
            cell_width_px: 10,
            line_height_px: 10,
            surface_padding_px: 0,
            cell_spacing_px: 0,
            kind: MouseEventKind::Press,
            button: MouseButton::Left,
            modifiers: winit::keyboard::ModifiersState::empty(),
        })
        .unwrap();

    assert!(result.handled);
    assert!(result.needs_redraw);
    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());
    assert!(
        renderer
            .frames
            .last()
            .unwrap()
            .lines
            .iter()
            .any(|line| line.contains("tmux manager | focus windows"))
    );
}

#[test]
fn tmux_manager_panel_mouse_press_focuses_visible_rows() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);

    assert_eq!(
        panel.handle_mouse_event(panel_mouse(MouseEventKind::Press, MouseButton::Left, 2)),
        TmuxManagerMouseOutcome::Consumed
    );
    assert_eq!(panel.focus(), TmuxManagerFocus::Windows);

    assert_eq!(
        panel.handle_mouse_event(panel_mouse(MouseEventKind::Press, MouseButton::Left, 3)),
        TmuxManagerMouseOutcome::Consumed
    );
    assert_eq!(panel.focus(), TmuxManagerFocus::Panes);

    assert_eq!(
        panel.handle_mouse_event(panel_mouse(MouseEventKind::Press, MouseButton::Left, 4)),
        TmuxManagerMouseOutcome::Consumed
    );
    assert_eq!(panel.focus(), TmuxManagerFocus::Actions);
}

#[test]
fn tmux_manager_panel_mouse_ignores_non_selection_events() {
    let snapshot = manager_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);

    assert_eq!(
        panel.handle_mouse_event(panel_mouse(MouseEventKind::Drag, MouseButton::Left, 2)),
        TmuxManagerMouseOutcome::Ignored
    );
    assert_eq!(panel.focus(), TmuxManagerFocus::Sessions);

    assert_eq!(
        panel.handle_mouse_event(panel_mouse(MouseEventKind::Press, MouseButton::Right, 2)),
        TmuxManagerMouseOutcome::Ignored
    );
    assert_eq!(panel.focus(), TmuxManagerFocus::Sessions);
}

fn panel_mouse(kind: MouseEventKind, button: MouseButton, row: u16) -> MouseEvent {
    MouseEvent::new(kind, button, 0, row)
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
