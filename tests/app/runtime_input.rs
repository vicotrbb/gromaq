use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, NativeWindowMouseInput};
use gromaq::pty::ShellCommand;
use gromaq::{MouseButton, MouseEvent, MouseEventKind};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::support::MockPtySpawner;

#[test]
fn native_terminal_runtime_encodes_mouse_input_to_pty_when_reporting_is_enabled() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
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

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<0;3;2M"
    );
}

#[test]
fn native_terminal_runtime_encodes_default_mouse_protocol_to_pty() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
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

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[M #\""
    );
}

#[test]
fn native_terminal_runtime_encodes_window_mouse_modifiers_to_pty() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
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
            .send_window_mouse_input_event(NativeWindowMouseInput {
                x: 100.0,
                y: 150.0,
                window_width_px: 800,
                window_height_px: 400,
                kind: MouseEventKind::Press,
                button: MouseButton::Left,
                modifiers: ModifiersState::SHIFT.union(ModifiersState::CONTROL),
            })
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<20;3;2M"
    );
}

#[test]
fn native_terminal_runtime_maps_window_mouse_input_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
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
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Press,
                MouseButton::Left,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<0;3;2M"
    );
}

#[test]
fn native_terminal_runtime_maps_window_mouse_drag_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
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
        .push_back(b"\x1b[?1002h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

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
}

#[test]
fn native_terminal_runtime_maps_alternate_screen_window_mouse_drag_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
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
        .push_back(b"\x1b[?1049halt\x1b[?1002h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "alt");

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
}

#[test]
fn native_terminal_runtime_maps_alternate_screen_window_mouse_press_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
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
        .push_back(b"\x1b[?1049halt\x1b[?1000h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "alt");

    assert!(
        runtime
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Press,
                MouseButton::Left,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<0;3;2M"
    );
}

#[test]
fn native_terminal_runtime_maps_alternate_screen_window_mouse_release_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
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
        .push_back(b"\x1b[?1049halt\x1b[?1000h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "alt");

    assert!(
        runtime
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Release,
                MouseButton::Left,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<0;3;2m"
    );
}

#[test]
fn native_terminal_runtime_maps_alternate_screen_window_mouse_wheel_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
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
        .push_back(b"\x1b[?1049halt\x1b[?1000h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "alt");

    assert!(
        runtime
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Press,
                MouseButton::WheelDown,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<65;3;2M"
    );
}

#[test]
fn native_terminal_runtime_scrolls_scrollback_on_unreported_wheel() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 6,
        terminal_rows: 3,
        scrollback_lines: 8,
        pixel_width: 0,
        pixel_height: 0,
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
        .push_back(b"one\r\ntwo\r\nthree\r\nfour".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "two");
    assert_eq!(runtime.terminal().dump_grid().line_text(2), "four");

    assert!(
        runtime
            .send_window_mouse_input(
                5.0,
                5.0,
                60,
                30,
                MouseEventKind::Press,
                MouseButton::WheelUp,
            )
            .unwrap()
    );

    let scrolled = runtime.terminal().dump_grid();
    assert_eq!(scrolled.line_text(0), "one");
    assert_eq!(scrolled.line_text(1), "two");
    assert_eq!(scrolled.line_text(2), "three");
    assert!(!runtime.terminal().dump_cursor().visible);
    assert!(runtime.shell_session().unwrap().input.borrow().is_empty());

    assert!(
        runtime
            .send_window_mouse_input(
                5.0,
                5.0,
                60,
                30,
                MouseEventKind::Press,
                MouseButton::WheelDown,
            )
            .unwrap()
    );

    let live = runtime.terminal().dump_grid();
    assert_eq!(live.line_text(0), "two");
    assert_eq!(live.line_text(1), "three");
    assert_eq!(live.line_text(2), "four");
    assert!(runtime.terminal().dump_cursor().visible);
}

#[test]
fn native_terminal_runtime_scrolls_scrollback_on_shift_page_keys() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 6,
        terminal_rows: 3,
        scrollback_lines: 8,
        pixel_width: 0,
        pixel_height: 0,
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
        .push_back(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "four");
    assert_eq!(runtime.terminal().dump_grid().line_text(2), "six");

    assert!(
        runtime
            .send_winit_key_input(&Key::Named(NamedKey::PageUp), ModifiersState::SHIFT)
            .unwrap()
    );

    let scrolled = runtime.terminal().dump_grid();
    assert_eq!(scrolled.line_text(0), "two");
    assert_eq!(scrolled.line_text(1), "three");
    assert_eq!(scrolled.line_text(2), "four");
    assert!(!runtime.terminal().dump_cursor().visible);
    assert!(runtime.shell_session().unwrap().input.borrow().is_empty());

    assert!(
        runtime
            .send_winit_key_input(&Key::Named(NamedKey::PageDown), ModifiersState::SHIFT)
            .unwrap()
    );

    let live = runtime.terminal().dump_grid();
    assert_eq!(live.line_text(0), "four");
    assert_eq!(live.line_text(1), "five");
    assert_eq!(live.line_text(2), "six");
    assert!(runtime.terminal().dump_cursor().visible);
    assert!(runtime.shell_session().unwrap().input.borrow().is_empty());
}

#[test]
fn native_terminal_runtime_sends_shift_page_keys_to_alternate_screen_apps() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
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
        .push_back(b"\x1b[?1049halt".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "alt");

    assert!(
        runtime
            .send_winit_key_input(&Key::Named(NamedKey::PageUp), ModifiersState::SHIFT)
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[b"\x1b[5;2~".to_vec()]);
}

#[test]
fn native_terminal_runtime_leaves_shift_page_keys_unhandled_when_primary_scrollback_cannot_move() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();

    assert!(
        !runtime
            .send_winit_key_input(&Key::Named(NamedKey::PageUp), ModifiersState::SHIFT)
            .unwrap()
    );

    assert!(runtime.shell_session().unwrap().input.borrow().is_empty());
}

#[test]
fn native_terminal_runtime_maps_alternate_screen_window_mouse_motion_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
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
        .push_back(b"\x1b[?1049halt\x1b[?1003h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "alt");

    assert!(
        runtime
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Motion,
                MouseButton::None,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<35;3;2M"
    );
}

#[test]
fn native_terminal_runtime_maps_window_mouse_motion_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
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
        .push_back(b"\x1b[?1003h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Motion,
                MouseButton::None,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<35;3;2M"
    );
}
