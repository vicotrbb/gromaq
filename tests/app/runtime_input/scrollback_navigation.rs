use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use gromaq::pty::ShellCommand;
use gromaq::{MouseButton, MouseEventKind};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::support::MockPtySpawner;

#[test]
fn native_terminal_runtime_scrolls_scrollback_on_unreported_wheel() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 6,
        terminal_rows: 3,
        scrollback_lines: 8,
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

    assert!(
        !runtime
            .send_winit_key_input(&Key::Named(NamedKey::PageUp), ModifiersState::SHIFT)
            .unwrap()
    );

    assert!(runtime.shell_session().unwrap().input.borrow().is_empty());
}
