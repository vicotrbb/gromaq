use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use gromaq::pty::ShellCommand;
use gromaq::{MouseButton, MouseEvent, MouseEventKind};

use crate::support::MockPtySpawner;

#[test]
fn native_terminal_runtime_encodes_sgr_mouse_press_and_release_to_pty() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
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
        .push_back(b"\x1b[?1000h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_mouse_input(MouseEvent::new(
                MouseEventKind::Press,
                MouseButton::Left,
                2,
                1
            ))
            .unwrap()
    );
    assert!(
        runtime
            .send_mouse_input(MouseEvent::new(
                MouseEventKind::Release,
                MouseButton::Left,
                2,
                1,
            ))
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().len(), 2);
    assert_eq!(session.input.borrow()[0].as_slice(), b"\x1b[<0;3;2M");
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<0;3;2m"
    );
}

#[test]
fn native_terminal_runtime_encodes_default_mouse_protocol_press_and_release_to_pty() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
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
        .push_back(b"\x1b[?1000h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_mouse_input(MouseEvent::new(
                MouseEventKind::Press,
                MouseButton::Left,
                2,
                1,
            ))
            .unwrap()
    );
    assert!(
        runtime
            .send_mouse_input(MouseEvent::new(
                MouseEventKind::Release,
                MouseButton::Left,
                2,
                1,
            ))
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().len(), 2);
    assert_eq!(session.input.borrow()[0].as_slice(), b"\x1b[M #\"");
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[M##\""
    );
}

#[test]
fn native_terminal_runtime_encodes_x10_mouse_press_only_to_pty() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
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
        .push_back(b"\x1b[?9h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_mouse_input(MouseEvent::new(
                MouseEventKind::Press,
                MouseButton::Left,
                2,
                1,
            ))
            .unwrap()
    );
    assert!(
        !runtime
            .send_mouse_input(MouseEvent::new(
                MouseEventKind::Release,
                MouseButton::Left,
                2,
                1,
            ))
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().len(), 1);
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[M #\""
    );
}
