use super::*;

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
