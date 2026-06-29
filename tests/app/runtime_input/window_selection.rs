use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, NativeWindowMouseInput};
use gromaq::pty::ShellCommand;
use gromaq::{MemoryClipboard, MouseButton, MouseEventKind, SelectionRange};
use winit::keyboard::ModifiersState;

use crate::support::{MockPtySession, MockPtySpawner};

fn runtime_with_output(
    terminal_cols: u16,
    terminal_rows: u16,
    scrollback_lines: usize,
    output: &[u8],
) -> NativeTerminalRuntime<MockPtySession> {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols,
        terminal_rows,
        scrollback_lines,
        pixel_width: 0,
        pixel_height: 0,
        cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
        cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(output.to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    runtime
}

fn mouse_input(
    x: f64,
    y: f64,
    kind: MouseEventKind,
    button: MouseButton,
) -> NativeWindowMouseInput {
    NativeWindowMouseInput {
        x,
        y,
        window_width_px: 100,
        window_height_px: 30,
        cell_width_px: 10,
        line_height_px: 10,
        surface_padding_px: 0,
        cell_spacing_px: 0,
        kind,
        button,
        modifiers: ModifiersState::empty(),
    }
}

#[test]
fn native_terminal_runtime_selects_visible_text_with_left_drag() {
    let mut runtime = runtime_with_output(10, 3, 100, b"alpha\r\nbravo\r\ncharlie");

    assert!(
        runtime
            .send_window_mouse_input_event(mouse_input(
                15.0,
                5.0,
                MouseEventKind::Press,
                MouseButton::Left
            ))
            .unwrap()
    );
    assert!(
        runtime
            .send_window_mouse_input_event(mouse_input(
                45.0,
                15.0,
                MouseEventKind::Drag,
                MouseButton::Left
            ))
            .unwrap()
    );
    assert!(
        runtime
            .send_window_mouse_input_event(mouse_input(
                45.0,
                15.0,
                MouseEventKind::Release,
                MouseButton::Left
            ))
            .unwrap()
    );

    assert_eq!(
        runtime.terminal().dump_grid().selection,
        Some(SelectionRange::new((0, 1), (1, 4)))
    );
    assert_eq!(runtime.terminal().copy_selection().unwrap(), "lpha\nbravo");
    assert!(runtime.shell_session().unwrap().input.borrow().is_empty());
}

#[test]
fn native_terminal_runtime_copy_shortcut_copies_drag_selection_without_pty_input() {
    let mut runtime = runtime_with_output(10, 3, 100, b"alpha\r\nbravo\r\ncharlie");

    assert!(
        runtime
            .send_window_mouse_input_event(mouse_input(
                0.0,
                0.0,
                MouseEventKind::Press,
                MouseButton::Left
            ))
            .unwrap()
    );
    assert!(
        runtime
            .send_window_mouse_input_event(mouse_input(
                45.0,
                0.0,
                MouseEventKind::Drag,
                MouseButton::Left
            ))
            .unwrap()
    );
    assert!(
        runtime
            .send_window_mouse_input_event(mouse_input(
                45.0,
                0.0,
                MouseEventKind::Release,
                MouseButton::Left
            ))
            .unwrap()
    );

    let mut clipboard = MemoryClipboard::default();
    assert!(runtime.copy_selection_to_clipboard(&mut clipboard));
    assert_eq!(clipboard.read_text().unwrap(), "alpha");
    assert!(runtime.shell_session().unwrap().input.borrow().is_empty());
}

#[test]
fn native_terminal_runtime_selects_displayed_scrollback_text_with_left_drag() {
    let mut runtime = runtime_with_output(10, 3, 8, b"one\r\ntwo\r\nthree\r\nfour\r\nfive");
    assert!(
        runtime
            .send_window_mouse_input_event(mouse_input(
                5.0,
                5.0,
                MouseEventKind::Press,
                MouseButton::WheelUp
            ))
            .unwrap()
    );

    assert!(
        runtime
            .send_window_mouse_input_event(mouse_input(
                0.0,
                0.0,
                MouseEventKind::Press,
                MouseButton::Left
            ))
            .unwrap()
    );
    assert!(
        runtime
            .send_window_mouse_input_event(mouse_input(
                45.0,
                10.0,
                MouseEventKind::Drag,
                MouseButton::Left
            ))
            .unwrap()
    );

    assert_eq!(runtime.terminal().copy_selection().unwrap(), "two\nthree");
    assert!(runtime.shell_session().unwrap().input.borrow().is_empty());
}

#[test]
fn native_terminal_runtime_reports_redraw_needed_for_scrollback_and_selection_changes() {
    let mut runtime = runtime_with_output(10, 3, 8, b"one\r\ntwo\r\nthree\r\nfour");

    let scrolled = runtime
        .send_window_mouse_input_event_result(mouse_input(
            5.0,
            5.0,
            MouseEventKind::Press,
            MouseButton::WheelUp,
        ))
        .unwrap();
    let pressed = runtime
        .send_window_mouse_input_event_result(mouse_input(
            0.0,
            0.0,
            MouseEventKind::Press,
            MouseButton::Left,
        ))
        .unwrap();
    let dragged = runtime
        .send_window_mouse_input_event_result(mouse_input(
            45.0,
            10.0,
            MouseEventKind::Drag,
            MouseButton::Left,
        ))
        .unwrap();

    assert!(scrolled.handled);
    assert!(scrolled.needs_redraw);
    assert!(pressed.handled);
    assert!(!pressed.needs_redraw);
    assert!(dragged.handled);
    assert!(dragged.needs_redraw);
}

#[test]
fn native_terminal_runtime_keeps_reported_mouse_drag_for_terminal_app() {
    let mut runtime = runtime_with_output(80, 20, 100, b"\x1b[?1002h\x1b[?1006h");

    assert!(
        runtime
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Drag,
                MouseButton::Left,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<32;3;2M"
    );
    assert_eq!(runtime.terminal().dump_grid().selection, None);
}
