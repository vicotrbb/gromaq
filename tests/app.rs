use std::path::PathBuf;
use std::time::Duration;

use gromaq::app::{
    NativeAppAction, NativeAppConfig, NativeAppLifecycle, NativePtyResize,
    NativeRuntimePerfSnapshot, NativeRuntimeStateSnapshot, NativeTerminalRuntime,
    NativeTerminalRuntimeConfig,
};
use gromaq::pty::ShellCommand;
use gromaq::{MemoryClipboard, SelectionRange};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

#[path = "app/config.rs"]
mod config;
#[path = "app/input_mapping.rs"]
mod input_mapping;
#[path = "app/lifecycle.rs"]
mod lifecycle;
#[path = "app/presentation.rs"]
mod presentation;
#[path = "app/rendering.rs"]
mod rendering;
#[path = "app/runtime_input.rs"]
mod runtime_input;
#[path = "app/support.rs"]
mod support;
#[path = "app/surface.rs"]
mod surface;

use support::{MockFrameRenderer, MockPtySession, MockPtySpawner};

#[test]
fn native_terminal_runtime_pumps_output_before_scheduling_redraw() {
    let spawner = MockPtySpawner::default();
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig::default());
    lifecycle.on_window_created();
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

    let action = runtime
        .pump_output_and_schedule_redraw(&mut lifecycle)
        .unwrap();

    assert_eq!(action, NativeAppAction::RequestRedraw);
    assert_eq!(lifecycle.redraw_requests(), 1);
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "hello");

    let idle_action = runtime
        .pump_output_and_schedule_redraw(&mut lifecycle)
        .unwrap();

    assert_eq!(idle_action, NativeAppAction::None);
    assert_eq!(lifecycle.redraw_requests(), 1);
}

#[test]
fn native_terminal_runtime_renders_dirty_terminal_frame_once() {
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
    runtime.pump_pty_output().unwrap();
    let mut renderer = MockFrameRenderer::default();

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());
    assert_eq!(renderer.frames.len(), 1);
    assert_eq!(renderer.frames[0].first_line, "hello");
    assert_eq!(renderer.frames[0].cursor.row, 1);
    assert!(!renderer.frames[0].dirty_regions.is_empty());

    assert!(!runtime.render_terminal_frame(&mut renderer).unwrap());
    assert_eq!(renderer.frames.len(), 1);
}

#[test]
fn native_terminal_runtime_starts_shell_pty_once_and_keeps_session() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 100,
        terminal_rows: 30,
        scrollback_lines: 2_000,
        pixel_width: 900,
        pixel_height: 600,
        cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
        cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: vec!["-l".into()],
            cwd: None,
        },
    })
    .unwrap();

    assert_eq!(runtime.terminal().dump_grid().cols, 100);
    assert_eq!(runtime.terminal().dump_grid().rows, 30);

    runtime.start_shell(&spawner).unwrap();
    runtime.start_shell(&spawner).unwrap();

    let configs = spawner.configs.borrow();
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].cols, 100);
    assert_eq!(configs[0].rows, 30);
    assert_eq!(configs[0].pixel_width, 900);
    assert_eq!(configs[0].pixel_height, 600);
    assert_eq!(configs[0].shell.program, "/bin/sh");
    assert_eq!(configs[0].shell.args, vec!["-l"]);
    assert!(runtime.has_shell_session());
}

#[test]
fn native_terminal_runtime_restarts_shell_with_updated_command() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 24,
        scrollback_lines: 1_000,
        pixel_width: 800,
        pixel_height: 480,
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
        .restart_shell(
            ShellCommand {
                program: "/bin/zsh".into(),
                args: vec!["-l".into()],
                cwd: Some("/tmp".into()),
            },
            &spawner,
        )
        .unwrap();

    let configs = spawner.configs.borrow();
    assert_eq!(configs.len(), 2);
    assert_eq!(configs[1].cols, 80);
    assert_eq!(configs[1].rows, 24);
    assert_eq!(configs[1].pixel_width, 800);
    assert_eq!(configs[1].pixel_height, 480);
    assert_eq!(configs[1].shell.program, PathBuf::from("/bin/zsh"));
    assert_eq!(configs[1].shell.args, vec![PathBuf::from("-l")]);
    assert_eq!(configs[1].shell.cwd, Some(PathBuf::from("/tmp")));
    assert_eq!(runtime.config().shell, configs[1].shell);
    assert!(runtime.has_shell_session());
}

#[test]
fn native_terminal_runtime_pumps_pty_output_and_writes_input() {
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

    let bytes = runtime.pump_pty_output().unwrap();
    runtime.send_pty_input(b"pwd\n").unwrap();

    assert_eq!(bytes, 7);
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "hello");
    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[b"pwd\n".to_vec()]);
}

#[test]
fn native_runtime_perf_metrics_track_io_resize_and_render_boundaries() {
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
    assert_eq!(
        runtime.dump_runtime_perf_metrics(),
        NativeRuntimePerfSnapshot::default()
    );
    runtime.start_shell(&spawner).unwrap();
    runtime.pump_pty_output().unwrap();
    runtime
        .send_winit_key_input(&Key::Character("c".into()), ModifiersState::CONTROL)
        .unwrap();
    runtime.send_paste_text("ab").unwrap();
    runtime.send_committed_text("é").unwrap();
    runtime
        .resize_terminal(NativePtyResize {
            cols: 10,
            rows: 6,
            pixel_width: 800,
            pixel_height: 480,
        })
        .unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[6n".to_vec());
    runtime.pump_pty_output().unwrap();
    let mut renderer = MockFrameRenderer {
        render_delay: Duration::from_millis(1),
        ..MockFrameRenderer::default()
    };
    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());
    assert!(!runtime.render_terminal_frame(&mut renderer).unwrap());
    let rendered_dirty_regions = renderer.frames[0].dirty_regions.len() as u64;
    let rendered_dirty_cells = renderer.frames[0]
        .dirty_regions
        .iter()
        .map(|region| u64::from(region.rows) * u64::from(region.cols))
        .sum::<u64>();

    let metrics = runtime.dump_runtime_perf_metrics();
    assert_eq!(metrics.pty_output_batches, 2);
    assert_eq!(metrics.pty_output_bytes, 11);
    assert_eq!(metrics.pty_response_writes, 1);
    assert!(!runtime.shell_session().unwrap().input.borrow()[3].is_empty());
    assert_eq!(
        metrics.pty_response_bytes,
        runtime.shell_session().unwrap().input.borrow()[3].len() as u64
    );
    assert_eq!(metrics.pty_input_writes, 3);
    assert_eq!(metrics.pty_input_bytes, 5);
    assert_eq!(metrics.native_key_inputs, 1);
    assert_eq!(metrics.paste_bytes, 2);
    assert_eq!(metrics.committed_text_bytes, 2);
    assert_eq!(metrics.resize_events, 1);
    assert_eq!(metrics.render_attempts, 2);
    assert_eq!(metrics.rendered_frames, 1);
    assert_eq!(metrics.rendered_dirty_regions, rendered_dirty_regions);
    assert_eq!(metrics.rendered_dirty_cells, rendered_dirty_cells);
    assert_eq!(metrics.rendered_dirty_cells_max, rendered_dirty_cells);
    assert_eq!(metrics.clean_frame_skips, 1);
    assert_eq!(metrics.render_time_samples, 1);
    assert!(metrics.render_time_total_ns >= 1_000_000);
    assert_eq!(metrics.render_time_avg_ns, metrics.render_time_total_ns);
    assert!(metrics.render_time_max_ns >= 1_000_000);
    assert!(metrics.render_time_p95_ns >= metrics.render_time_max_ns);
    assert!(metrics.render_time_total_ns >= metrics.render_time_max_ns);
    assert_eq!(metrics.input_to_render_samples, 1);
    assert_eq!(
        metrics.input_to_render_avg_ns,
        metrics.input_to_render_total_ns
    );
    assert!(metrics.input_to_render_total_ns >= metrics.input_to_render_max_ns);
    assert!(metrics.input_to_render_p95_ns >= metrics.input_to_render_max_ns);
}

#[test]
fn native_runtime_state_snapshot_reports_bounded_scrollback_footprint() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 8,
        terminal_rows: 2,
        scrollback_lines: 3,
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

    assert_eq!(
        runtime.dump_runtime_state_snapshot(),
        NativeRuntimeStateSnapshot {
            terminal_cols: 8,
            terminal_rows: 2,
            visible_cells: 16,
            scrollback_limit: 3,
            scrollback_lines: 0,
            scrollback_cell_rows: 0,
            scrollback_cells: 0,
            scrollback_cell_limit: 24,
        }
    );

    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\n".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    let state = runtime.dump_runtime_state_snapshot();
    assert_eq!(state.terminal_cols, 8);
    assert_eq!(state.terminal_rows, 2);
    assert_eq!(state.visible_cells, 16);
    assert_eq!(state.scrollback_limit, 3);
    assert_eq!(state.scrollback_lines, 3);
    assert_eq!(state.scrollback_cell_rows, 3);
    assert!(state.scrollback_cells <= state.scrollback_cell_limit);

    runtime
        .resize_terminal(NativePtyResize {
            cols: 5,
            rows: 4,
            pixel_width: 500,
            pixel_height: 320,
        })
        .unwrap();

    let resized = runtime.dump_runtime_state_snapshot();
    assert_eq!(resized.terminal_cols, 5);
    assert_eq!(resized.terminal_rows, 4);
    assert_eq!(resized.visible_cells, 20);
    assert_eq!(resized.scrollback_limit, 3);
    assert_eq!(resized.scrollback_cell_limit, 15);
    assert!(resized.scrollback_lines <= resized.scrollback_limit);
    assert!(resized.scrollback_cells <= resized.scrollback_cell_limit);
}

#[test]
fn native_terminal_runtime_writes_terminal_status_responses_to_pty() {
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
    runtime.pump_pty_output().unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[3;5H\x1b[6n\x1b[5n".to_vec());

    let bytes = runtime.pump_pty_output().unwrap();

    assert_eq!(bytes, 14);
    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().as_slice(),
        &[b"\x1b[3;5R\x1b[0n".to_vec()]
    );
}

#[test]
fn native_terminal_runtime_writes_device_attribute_responses_to_pty() {
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
    runtime.pump_pty_output().unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[c".to_vec());

    let bytes = runtime.pump_pty_output().unwrap();

    assert_eq!(bytes, 3);
    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[b"\x1b[?1;2c".to_vec()]);
}

#[test]
fn native_terminal_runtime_writes_secondary_device_attribute_responses_to_pty() {
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
    runtime.pump_pty_output().unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[>c".to_vec());

    let bytes = runtime.pump_pty_output().unwrap();

    assert_eq!(bytes, 4);
    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().as_slice(),
        &[b"\x1b[>0;1;0c".to_vec()]
    );
}

#[test]
fn native_terminal_runtime_resizes_terminal_and_pty_session() {
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
        .resize_terminal(NativePtyResize {
            cols: 10,
            rows: 6,
            pixel_width: 800,
            pixel_height: 480,
        })
        .unwrap();

    assert_eq!(runtime.terminal().dump_grid().cols, 10);
    assert_eq!(runtime.terminal().dump_grid().rows, 6);
    assert_eq!(runtime.config().terminal_cols, 10);
    assert_eq!(runtime.config().terminal_rows, 6);
    assert_eq!(runtime.config().pixel_width, 800);
    assert_eq!(runtime.config().pixel_height, 480);
    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.resizes.borrow().as_slice(),
        &[NativePtyResize {
            cols: 10,
            rows: 6,
            pixel_width: 800,
            pixel_height: 480,
        }]
    );
}

#[test]
fn native_terminal_runtime_updates_pixel_size_report_after_resize() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 900,
        pixel_height: 600,
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
    runtime.pump_pty_output().unwrap();
    runtime
        .resize_terminal(NativePtyResize {
            cols: 10,
            rows: 6,
            pixel_width: 800,
            pixel_height: 480,
        })
        .unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[14t".to_vec());

    let bytes = runtime.pump_pty_output().unwrap();

    assert_eq!(bytes, 5);
    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().as_slice(),
        &[b"\x1b[4;480;800t".to_vec()]
    );
}

#[test]
fn native_terminal_runtime_encodes_winit_key_input_to_pty() {
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
        runtime
            .send_winit_key_input(&Key::Character("c".into()), ModifiersState::CONTROL)
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[vec![0x03]]);
}

#[test]
fn native_terminal_runtime_uses_application_cursor_key_mode_for_arrows() {
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
        .push_back(b"\x1b[?1h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_winit_key_input(&Key::Named(NamedKey::ArrowUp), ModifiersState::empty())
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[b"\x1bOA".to_vec()]);
}

#[test]
fn native_terminal_runtime_returns_to_normal_cursor_key_mode_for_arrows() {
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
        .push_back(b"\x1b[?1h\x1b[?1l".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_winit_key_input(&Key::Named(NamedKey::ArrowUp), ModifiersState::empty())
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[b"\x1b[A".to_vec()]);
}

#[test]
fn native_terminal_runtime_uses_application_keypad_mode_for_numpad_keys() {
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
        .push_back(b"\x1b[?66h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_winit_key_event_input(
                &Key::Character("1".into()),
                Some(PhysicalKey::Code(KeyCode::Numpad1)),
                ModifiersState::empty(),
            )
            .unwrap()
    );
    assert!(
        runtime
            .send_winit_key_event_input(
                &Key::Named(NamedKey::Enter),
                Some(PhysicalKey::Code(KeyCode::NumpadEnter)),
                ModifiersState::empty(),
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().as_slice(),
        &[b"\x1bOq".to_vec(), b"\x1bOM".to_vec()]
    );
}

#[test]
fn native_terminal_runtime_sends_focus_reports_when_enabled() {
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
        .push_back(b"\x1b[?1004h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(runtime.send_focus_event(true).unwrap());
    assert!(runtime.send_focus_event(false).unwrap());

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().as_slice(),
        &[b"\x1b[I".to_vec(), b"\x1b[O".to_vec()]
    );
}

#[test]
fn native_terminal_runtime_suppresses_focus_reports_when_disabled() {
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
        .push_back(b"\x1b[?1004h\x1b[?1004l".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(!runtime.send_focus_event(true).unwrap());

    let session = runtime.shell_session().unwrap();
    assert!(session.input.borrow().is_empty());
}

#[test]
fn native_terminal_runtime_encodes_paste_text_to_pty() {
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
        .push_back(b"\x1b[?2004h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    runtime.send_paste_text("abc").unwrap();

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[200~abc\x1b[201~"
    );
}

#[test]
fn native_terminal_runtime_copies_selection_to_clipboard() {
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
    runtime.pump_pty_output().unwrap();
    runtime.set_selection(SelectionRange::new((0, 1), (0, 3)));
    let mut clipboard = MemoryClipboard::default();

    assert!(runtime.copy_selection_to_clipboard(&mut clipboard));
    assert_eq!(clipboard.read_text().as_deref(), Some("ell"));
}

#[test]
fn native_terminal_runtime_reads_clipboard_and_encodes_paste_to_pty() {
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
        .push_back(b"\x1b[?2004h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    let clipboard = MemoryClipboard::new("from clipboard");

    assert!(runtime.send_clipboard_paste(&clipboard).unwrap());

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[200~from clipboard\x1b[201~"
    );
}

#[test]
fn native_terminal_runtime_does_not_count_clipboard_paste_without_session() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
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
    let clipboard = MemoryClipboard::new("from clipboard");

    assert!(!runtime.send_clipboard_paste(&clipboard).unwrap());

    let metrics = runtime.dump_runtime_perf_metrics();
    assert_eq!(metrics.clipboard_pastes, 0);
    assert_eq!(metrics.paste_bytes, 0);
    assert!(!runtime.has_shell_session());
}

#[test]
fn native_terminal_runtime_syncs_osc52_clipboard_text_to_host_clipboard() {
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
        .push_back(b"\x1b]52;c;ZnJvbSBvc2M1Mg==\x07".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    let mut clipboard = MemoryClipboard::default();

    assert!(runtime.sync_terminal_clipboard(&mut clipboard));
    assert_eq!(clipboard.read_text().as_deref(), Some("from osc52"));
    assert!(!runtime.sync_terminal_clipboard(&mut clipboard));
}

#[test]
fn native_terminal_runtime_writes_committed_text_to_pty() {
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
        .push_back(b"\x1b[?2004h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    runtime.send_committed_text("olá").unwrap();

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        "olá".as_bytes()
    );
}

#[test]
fn native_terminal_runtime_ignores_empty_pty_input_writes() {
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

    runtime.send_pty_input(b"").unwrap();
    runtime.send_paste_text("").unwrap();
    runtime.send_committed_text("").unwrap();

    let session = runtime.shell_session().unwrap();
    assert!(session.input.borrow().is_empty());
    assert_eq!(
        runtime.dump_runtime_perf_metrics(),
        NativeRuntimePerfSnapshot::default()
    );
}
